use std::{collections::{HashMap, HashSet}, sync::LazyLock};

use indent::indent_by;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};

use crate::GameReadError;

#[derive(Clone)]
struct SectionedStrings {
    strings: Vec<String>,
    dscs: Vec<String>,
    is_strings_first: bool,
}

impl SectionedStrings {
    pub fn new(line: &str) -> Self {
        /// Regex composition:
        /// - **`".*?`** - Quotation followed by anything (lazy to not bleed into future quotes)
        /// - **`[^\\](?:\\\\)*`** - Even number of escapes (repeating double escapes anchored on the left by a non-escape)
        /// - **`"`** - Detect a real closing quotation
        /// 
        /// We can ignore falsly detecting quotes in comments, we deal with that later.
        static STRING_REGEX: LazyLock<Regex> = LazyLock::new(||
            Regex::new(r#"".*?[^\\](?:\\\\)*""#).unwrap()
        );

        let mut strings = STRING_REGEX.find_iter(line).peekable();
        let dscs = STRING_REGEX.split(line)
            .filter(|d| !d.is_empty());

        let is_strings_first = if let Some(s) = strings.peek() {
            s.start() == 0
        } else {
            false
        };

        Self {
            strings: strings
                .map(|s| s.as_str().to_string())
                .collect(),
            dscs: dscs
                .map(|d| d.to_string())
                .collect(),
            is_strings_first
        }
    }
}

struct ReplacedRefsDsc {
    replaced_dsc: String,
    referenced_subs: HashSet<String>,
    mutated_state: Option<String>,
    has_comment: bool,
}

impl ReplacedRefsDsc {
    pub fn try_new(dsc: &str, src_line: &str, mut mutated_state: Option<String>) -> Result<Self, GameReadError> {
        #[derive(Clone, Copy)]
        enum Rune {
            Mutation,
            Subscription,
            Snapshot,
        }

        let mut dsc = dsc.to_string();
        // To prevent needing to do cleanup after the loop finishes
        dsc.push('\0');

        let mut referenced_subs = HashSet::new();
        let mut replaced_dsc = String::new();
        let mut has_comment = false;

        let mut window_left = 0;
        let mut runing = false;
        let mut rune = None;

        for (i, c) in dsc.char_indices() {
            if c == '$' {
                if runing {
                    // Escaped double $$, replace with single $
                    runing = false;
                    replaced_dsc.push('$');
                    // Slide window past the $$. CANNOT BE OUT-OF-BOUNDS because of null-terminator
                    window_left = i + 1;
                } else {
                    runing = true;
                    replaced_dsc.push_str(&dsc[window_left..i]);
                }
            } else if runing {
                // Identify what rune we're using
                if c == '<' {
                    rune = Some(Rune::Subscription);
                    runing = false;
                    // CANNOT BE OUT-OF-BOUNDS because of null-terminator
                    window_left = i + 1;
                } else if c == '>' {
                    rune = Some(Rune::Mutation);
                    runing = false;
                    // CANNOT BE OUT-OF-BOUNDS because of null-terminator
                    window_left = i + 1;
                } else if matches!(c, 'a'..='z' | 'A'..='Z' | '_') {
                    rune = Some(Rune::Snapshot);
                    runing = false;
                    window_left = i;
                } else {
                    if c == ' ' {
                        return Err(GameReadError::DynamicString(
                            format!("floating '$' found in `{src_line}`")
                        ));
                    } else if c == '\0' {
                        // End of the DSC section reached
                        return Err(GameReadError::DynamicString(
                            format!("unknown rune '$\"' found in `{src_line}`")
                        ));
                    } else {
                        return Err(GameReadError::DynamicString(
                            format!("unknown rune '${c}' found in `{src_line}`")
                        ));
                    }
                }
            } else if let Some(r) = rune {
                // Identify the name of the state we're accessing
                if !matches!(c, 'a'..='z' | 'A'..='Z' | '_') {
                    if i == window_left {
                        return Err(GameReadError::DynamicString(
                            format!("floating rune found in `{src_line}`")
                        ));
                    } else if !c.is_ascii_digit() {
                        // We found the end of the state name
                        let state = &dsc[window_left..i];
                        window_left = i;
                        rune = None;

                        match r {
                            Rune::Mutation => {
                                if let Some(m) = &mutated_state {
                                    if m != state {
                                        return Err(GameReadError::DynamicString(
                                            format!("more than one mutated state found in `{src_line}`")
                                        ));
                                    }
                                } else {
                                    mutated_state = Some(state.to_string());
                                }

                                replaced_dsc.push_str(state);
                            },
                            Rune::Subscription => {
                                // Don't change anything yet, just remember it for later
                                replaced_dsc.push_str(&format!("$<{state}"));
                                referenced_subs.insert(state.to_string());
                            },
                            Rune::Snapshot => {
                                replaced_dsc.push_str(&format!("State.snapshot(slf.states.{state})"));
                            },
                        }
                    }
                }
            } else if c == '/' && &dsc[i+1..=i+1] == "/" {
                // Comment detected
                has_comment = true;
                replaced_dsc.push_str(&dsc[window_left..dsc.len()-1]);
                break;
            } else if c == '\0' {
                // End of the the DSC section reached while not reading anything special
                replaced_dsc.push_str(&dsc[window_left..dsc.len()-1]);
                break;
            }
        }

        Ok(Self {
            replaced_dsc,
            referenced_subs,
            mutated_state,
            has_comment,
        })
    }
}

struct HalfCompiledDynamicString {
    line_sections: Vec<(Option<String>, SectionedStrings)>,
    referenced_subs: HashSet<String>,
}

impl HalfCompiledDynamicString {
    pub fn from_src(src: &str, is_bind_string: bool) -> Result<Self, GameReadError> {
        /// Regex composition:
        /// - **`^<space>*\$>`** - Line begins with the literal `$>`
        /// - **CAPTURE** - Get the state name
        ///   - **`[a-zA-Z_][a-zA-Z0-9_]*`** - Any valid state name
        /// - **`<space>*:`** - Any amount of spaces between the state name and the colon
        /// - **`<space>*`** - Any amount of spaces between the colon and the mutation itself
        /// - **CAPTURE** - Get the entire rest of the line
        ///   - **`.*`** - Anything
        static MUTATION_REGEX: LazyLock<Regex> = LazyLock::new(||
            Regex::new(r#"^ *\$>([a-zA-Z_][a-zA-Z0-9_]*) *: *(.*)"#).unwrap()
        );

        let mut line_sections = Vec::new();
        let mut lines: std::str::Lines<'_> = src.lines();
        // Skip the first line (only contains 'callback/bind')
        lines.next();
        // Only actually used if `is_bind_string` is true
        let mut referenced_subs = HashSet::new();

        // Compile line-by-line, to join together afterwards
        for mut line in lines {
            let mut compiled_dscs = Vec::new();
            let mut mutated_state = None;
            let mut has_comment = false;

            let _line;
            if let Some(captures) = MUTATION_REGEX.captures(line) {
                mutated_state = Some(captures[1].to_string());
                _line = captures[2].to_string();
                line = &_line;
            }

            let sections = SectionedStrings::new(line);

            // Compile each DSC section separately, ignore actual strings
            for dsc in sections.dscs {
                if has_comment {
                    // A comment has already been found on this line, so don't compile anymore
                    compiled_dscs.push(dsc);
                    continue;
                }

                let compiled_dsc = ReplacedRefsDsc::try_new(&dsc, line, mutated_state)?;

                let ReplacedRefsDsc {
                    replaced_dsc: _replaced_dsc,
                    referenced_subs: _referenced_subs,
                    mutated_state: _mutated_state,
                    has_comment: _has_comment,
                } = compiled_dsc;

                if is_bind_string {
                    referenced_subs.extend(_referenced_subs);
                } else if !_referenced_subs.is_empty() {
                    return Err(GameReadError::DynamicString(
                        format!("state subscription rune $< found in dynamic callback string `{dsc}`")
                    ));
                }

                has_comment = _has_comment;
                mutated_state = _mutated_state;
                compiled_dscs.push(_replaced_dsc);
            }

            line_sections.push((mutated_state, SectionedStrings {
                strings: sections.strings,
                dscs: compiled_dscs,
                is_strings_first: sections.is_strings_first,
            }));
        }

        Ok(Self {
            line_sections,
            referenced_subs,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct DynamicCallbackString {
    source: String,
}

// For deserialization from String to work
impl TryFrom<String> for DynamicCallbackString {
    /// Used to signal to serde that this is not the correct variant of `DynamicString`.
    /// The boolean value itself does not matter.
    type Error = bool;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.lines().next() != Some("callback") {
            // This is not a callback
            return Err(true);
        }

        Ok(Self {
            source: value
        })
    }
}

// For serialization into String to work
impl From<DynamicCallbackString> for String {
    fn from(value: DynamicCallbackString) -> Self {
        value.source
    }
}

impl DynamicCallbackString {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn compile(&self, data_var: &str) -> Result<String, GameReadError> {
        let mut compiled_lines = Vec::new();
        let half_compiled = HalfCompiledDynamicString::from_src(&self.source, false)?;
        for (mutated_state, sections) in half_compiled.line_sections {
            // Join the sections back into a single compiled string
            let compiled_line = if sections.is_strings_first {
                itertools::interleave(sections.strings, sections.dscs)
            } else {
                itertools::interleave(sections.dscs, sections.strings)
            }
                .collect::<String>();
            compiled_lines.push(if let Some(state) = mutated_state {
                indoc::formatdoc! {r#"
                    State.mutate(slf.states.{state}, ({state}) =>
                        {compiled_line}
                    );
                "#}.trim_end().to_string()
            } else {
                compiled_line
            });
        }

        let compiled_result = indent_by(4, compiled_lines.join("\n"));

        Ok(indoc::formatdoc! {r#"
                {data_var} = () => {{
                    {compiled_result}
                }}
            "#}.trim_end().to_string()
        )
    }
}

pub enum SetData {
    Variable(String),
    Function(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct DynamicBindString {
    source: String,
}

// For deserialization from String to work
impl TryFrom<String> for DynamicBindString {
    /// Used to signal to serde that this is not the correct variant of `DynamicString`.
    /// The boolean value itself does not matter.
    type Error = bool;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.lines().next() != Some("bind") {
            // This is not a binding
            return Err(true);
        }

        Ok(Self {
            source: value
        })
    }
}

// For serialization into String to work
impl From<DynamicBindString> for String {
    fn from(value: DynamicBindString) -> Self {
        value.source
    }
}

impl DynamicBindString {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn compile(&self, data_var: SetData) -> Result<HashMap<String, String>, GameReadError> {
        /// Regex composition:
        /// - **`\$<`** - State subscription rune `$<`
        /// - **CAPTURE** - Get the state name
        ///   - **`[a-zA-Z_][a-zA-Z0-9_]*`** - Any valid state name
        static SUBSCRIBE_REGEX: LazyLock<Regex> = LazyLock::new(||
            Regex::new(r#"\$<([a-zA-Z_][a-zA-Z0-9_]*)"#).unwrap()
        );

        fn finish_sectioned_line(data_var: &SetData, sub_state: &str, half_compiled: &HalfCompiledDynamicString, is_simple_binding: bool) -> String {
            let mut compiled_lines = Vec::new();

            for (mutated_state, sections) in half_compiled.line_sections.clone() {
                // Replace the state subscriptions still remaining in the sectioned dscs
                let complete_dscs = sections.dscs.into_iter()
                    .map(|mut dsc| {
                        if sub_state != "" {
                            dsc = dsc.replace(&format!("$<{sub_state}"), &sub_state);
                        }
                        SUBSCRIBE_REGEX.replace_all(&dsc, "State.snapshot(slf.states.$1)").into_owned()
                    })
                    .collect::<Vec<_>>();

                // Join the sections back into a single compiled string
                let compiled_line = if sections.is_strings_first {
                    itertools::interleave(sections.strings, complete_dscs)
                } else {
                    itertools::interleave(complete_dscs, sections.strings)
                }
                    .collect::<String>();
                compiled_lines.push(if let Some(state) = mutated_state {
                    indoc::formatdoc! {r#"
                        State.mutate(slf.states.{state}, ({state}) =>
                            {compiled_line}
                        );
                    "#}.trim_end().to_string()
                } else {
                    compiled_line
                });
            }
    
            let compiled_result = indent_by(4, compiled_lines.join("\n"));
            if is_simple_binding {
                match data_var {
                    SetData::Variable(v) => format!("{v} = {compiled_result};"),
                    SetData::Function(f) => format!("{f}({compiled_result});"),
                }
            } else {
                match data_var {
                    SetData::Variable(v) => indoc::formatdoc! {r#"
                        {v} = (() => {{
                            {compiled_result}
                        }})();
                    "#}.trim_end().to_string(),
                    SetData::Function(f) => indoc::formatdoc! {r#"
                        {f}((() => {{
                            {compiled_result}
                        }})());
                    "#}.trim_end().to_string(),
                }
            }
        }

        let mut compiled_bindings = HashMap::new();
        let is_simple_binding = {
            let mut lines = self.source.lines();
            lines.next();
            if let Some(second_line) = lines.next() && lines.next().is_none() {
                !second_line.starts_with("return")
            } else {
                false
            }
        };
        let half_compiled = HalfCompiledDynamicString::from_src(&self.source, true)?;

        for sub in &half_compiled.referenced_subs {
            compiled_bindings.insert(sub.clone(), finish_sectioned_line(
                &data_var,
                sub,
                &half_compiled,
                is_simple_binding
            ));
        }
        // Key "" is the value initialization of the variable in `data_var`
        compiled_bindings.insert(String::new(), finish_sectioned_line(
            &data_var,
            "",
            &half_compiled,
            is_simple_binding
        ));

        Ok(compiled_bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_callback_string() {
        /* Regular usage */
        let test_string = indoc::indoc! {r#"
            callback
            $>count + 1
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                State.mutate(slf.states.count, (count) =>
                    count + 1
                );
            }
        "#}.trim_end().to_string());

        /* With comment */
        let test_string = indoc::indoc! {r#"
            callback
            $>count + 1 // With comment and $>count
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                State.mutate(slf.states.count, (count) =>
                    count + 1 // With comment and $>count
                );
            }
        "#}.trim_end().to_string());

        /* With strings */
        let test_string = indoc::indoc! {r#"
            callback
            "`$>count` = " + $>count + "!"
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                State.mutate(slf.states.count, (count) =>
                    "`$>count` = " + count + "!"
                );
            }
        "#}.trim_end().to_string());

        /* With multiple mutates of same state */
        let test_string = indoc::indoc! {r#"
            callback
            $>count + ($>count + 1)
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                State.mutate(slf.states.count, (count) =>
                    count + (count + 1)
                );
            }
        "#}.trim_end().to_string());

        /* With escaped $ */
        let test_string = indoc::indoc! {r#"
            callback
            non$$state = 15;
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                non$state = 15;
            }
        "#}.trim_end().to_string());

        /* Mutate with no read */
        let test_string = indoc::indoc! {r#"
            callback
            $>display: "hello world"
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                State.mutate(slf.states.display, (display) =>
                    "hello world"
                );
            }
        "#}.trim_end().to_string());

        /* With state snapshot */
        let test_string = indoc::indoc! {r#"
            callback
            $>count + $other
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                State.mutate(slf.states.count, (count) =>
                    count + State.snapshot(slf.states.other)
                );
            }
        "#}.trim_end().to_string());

        /* With multiple lines */
        let test_string = indoc::indoc! {r#"
            callback
            variable += 8;
            $>count * variable
            $>display: "hello world"
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile("slf.element.onclick").unwrap();
        assert_eq!(compiled_js, indoc::indoc! {r#"
            slf.element.onclick = () => {
                variable += 8;
                State.mutate(slf.states.count, (count) =>
                    count * variable
                );
                State.mutate(slf.states.display, (display) =>
                    "hello world"
                );
            }
        "#}.trim_end().to_string());
    }

    #[test]
    fn test_dynamic_callback_string_errors() {
        /* Empty rune */
        let test_string = indoc::indoc! {r#"
            callback
            $ + 1
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        assert!(dbg!(dynamic_string.compile("slf.element.onclick")
            .unwrap_err()
            .to_string())
            .starts_with("floating '$' found in")
        );

        /* Unknown rune */
        let test_string = indoc::indoc! {r#"
            callback
            $+ 1
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        assert!(dbg!(dynamic_string.compile("slf.element.onclick")
            .unwrap_err()
            .to_string())
            .starts_with("unknown rune '$+' found in")
        );

        /* State starting with number */
        let test_string = indoc::indoc! {r#"
            callback
            $1state + 1
        "#}.trim_end().to_string();
        let dynamic_string = DynamicCallbackString::try_from(test_string).unwrap();
        assert!(dbg!(dynamic_string.compile("slf.element.onclick")
            .unwrap_err()
            .to_string())
            .starts_with("unknown rune '$1' found in")
        );
    }

    #[test]
    fn test_dynamic_binding_string() {
        /* Simple binding with static value */
        let test_string = indoc::indoc! {r#"
            bind
            "Display this string"
        "#}.trim_end().to_string();
        let dynamic_string = DynamicBindString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile(SetData::Variable("slf.element.innerHTML".to_string())).unwrap();
        let expected = HashMap::from([
            ("".to_string(), "slf.element.innerHTML = \"Display this string\";".to_string())
        ]);
        assert_eq!(compiled_js, expected);

        /* Simple binding with state subscription */
        let test_string = indoc::indoc! {r#"
            bind
            $<count
        "#}.trim_end().to_string();
        let dynamic_string = DynamicBindString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile(SetData::Variable("slf.element.innerHTML".to_string())).unwrap();
        let expected = HashMap::from([
            ("".to_string(), "slf.element.innerHTML = State.snapshot(slf.states.count);".to_string()),
            ("count".to_string(), "slf.element.innerHTML = count;".to_string())
        ]);
        assert_eq!(compiled_js, expected);

        /* Standard binding with static value */
        let test_string = indoc::indoc! {r#"
            bind
            return "Display this string";
        "#}.trim_end().to_string();
        let dynamic_string = DynamicBindString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile(SetData::Variable("slf.element.innerHTML".to_string())).unwrap();
        let expected = HashMap::from([
            ("".to_string(), indoc::indoc! {r#"
                slf.element.innerHTML = (() => {
                    return "Display this string";
                })();
            "#}.trim_end().to_string())
        ]);
        assert_eq!(compiled_js, expected);

        /* Standard binding with state subscription */
        let test_string = indoc::indoc! {r#"
            bind
            return $<count;
        "#}.trim_end().to_string();
        let dynamic_string = DynamicBindString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile(SetData::Variable("slf.element.innerHTML".to_string())).unwrap();
        let expected = HashMap::from([
            ("".to_string(), indoc::indoc! {r#"
                slf.element.innerHTML = (() => {
                    return State.snapshot(slf.states.count);
                })();
            "#}.trim_end().to_string()),
            ("count".to_string(), indoc::indoc! {r#"
                slf.element.innerHTML = (() => {
                    return count;
                })();
            "#}.trim_end().to_string())
        ]);
        assert_eq!(compiled_js, expected);

        /* With multiple state subscriptions */
        let test_string = indoc::indoc! {r#"
            bind
            $<count * $<mult
        "#}.trim_end().to_string();
        let dynamic_string = DynamicBindString::try_from(test_string).unwrap();
        let compiled_js = dynamic_string.compile(SetData::Variable("slf.element.innerHTML".to_string())).unwrap();
        let expected = HashMap::from([
            ("".to_string(), "slf.element.innerHTML = State.snapshot(slf.states.count) * State.snapshot(slf.states.mult);".to_string()),
            ("count".to_string(), "slf.element.innerHTML = count * State.snapshot(slf.states.mult);".to_string()),
            ("mult".to_string(), "slf.element.innerHTML = State.snapshot(slf.states.count) * mult;".to_string()),

        ]);
        assert_eq!(compiled_js, expected);
    }
}
