use std::{ffi::OsStr, fs, sync::{LazyLock, Mutex, atomic::{AtomicBool, Ordering}}};
use serde::Deserialize;

use crate::{*, pieces::*};

static DEFINED_CUSTOM_PIECE_TYPES: LazyLock<Mutex<HashMap<&'static OsStr, &'static CustomPieceType>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn init_custom_piece_types(dir: &str) -> Result<(), GameReadError> {
    for type_file in fs::read_dir(dir)? {
        let type_file = type_file?;
        let type_file_path = type_file.path();
        if !type_file_path.is_file() || type_file_path.extension() != Some(OsStr::new("yaml")) {
            continue;
        }

        let type_name = type_file_path
            .file_stem()
            .unwrap()
            .to_os_string()
            .leak();
        let custom_piece_type = Box::new(serde_saphyr::from_reader(
            std::fs::File::open(type_file_path)?
        )?);

        DEFINED_CUSTOM_PIECE_TYPES
            .lock()
            .map_err(|e| GameReadError::CustomPiece(
                format!("Could not save defined custom piece types: {e}")
            ))?
            .insert(type_name, Box::leak(custom_piece_type));
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CustomPieceInput {
    Callback {
        id: String,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        params: Vec<String>,
    },
    Bind {
        id: String,
        #[serde(default)]
        required: bool
    },
}

impl CustomPieceInput {
    fn required(&self) -> bool {
        match self {
            Self::Bind { required, .. } => *required,
            Self::Callback { required, .. } => *required,
        }
    }

    fn id(&self) -> &str {
        match self {
            Self::Bind { id, .. } => id,
            Self::Callback { id, .. } => id,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CustomPieceType {
    inputs: Vec<CustomPieceInput>,
    base: Box<Pieces>,
    #[serde(skip)]
    base_was_built: AtomicBool,
}

#[derive(Debug, Deserialize)]
pub struct RawCustomPiece {
    id: String,
    r#type: String,
    /// Value kept as `String`, and converted into a `DynamicCallbackString` or `DynamicBindString` for `CustomPiece`
    /// depending on what the `CustomPieceType`, determined by `self.type`, specifies
    inputs: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "RawCustomPiece")]
pub struct CustomPiece {
    id: String,
    r#type: &'static CustomPieceType,
    #[serde(default)]
    data: CustomPieceData,
}

impl TryFrom<RawCustomPiece> for CustomPiece {
    type Error = GameReadError;

    fn try_from(value: RawCustomPiece) -> Result<Self, Self::Error> {
        let r#type = DEFINED_CUSTOM_PIECE_TYPES
            .lock()
            .map_err(|e| GameReadError::CustomPiece(
                format!("Could not read defined custom piece types: {e}")
            ))?
            .get(&value.r#type.as_ref())
            .copied()
            .ok_or(GameReadError::CustomPiece(
                format!("Undefined custom piece type `{}`", value.r#type)
            ))?;

        let id = value.id;
        let mut og_inputs = value.inputs;
        let mut inputs = HashMap::new();

        // Collect all defined inputs, and make sure all required inputs are present
        for input in &r#type.inputs {
            let (name, value) = match og_inputs.remove_entry(input.id()) {
                Some((name, og_value)) => match input {
                    CustomPieceInput::Callback {params, ..} => (
                        name, 
                        DynamicString::Callback(DynamicCallbackString::from(og_value).with_params(params.clone()))
                    ),
                    CustomPieceInput::Bind {..} => (
                        name, 
                        DynamicString::Bind(DynamicBindString::from(og_value))
                    ),
                },
                None if input.required() => return Err(GameReadError::CustomPiece(
                    format!("Input `{}` required for custom piece type `{}` (piece '{id}')", input.id(), value.r#type)
                )),
                _ => continue
            };

            inputs.insert(name, value);
        }

        // Raise an error if any extra undefined inputs are present
        if !og_inputs.is_empty() {
            let extras = og_inputs.into_keys()
                .map(|i| format!("'{i}'"))
                .collect::<Vec<_>>()
                .join(", ");

            return Err(GameReadError::CustomPiece(
                format!("Unknown inputs {{{extras}}} found for piece '{id}', remove them")
            ));
        }

        Ok(Self {
            id,
            r#type,
            data: CustomPieceData::new(inputs)?,
        })
    }
}

impl Piece for CustomPiece {
    type Data = CustomPieceData;

    fn states(&self) -> &[State] {
        &[]
    }

    fn events(&self) -> &[Event] {
        &[]
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn piece_id(&self) -> String {
        String::new()
    }

    fn piece_name(&self) -> String {
        String::new()
    }

    fn html_tag(&self) -> String {
        String::new()
    }

    fn html_skeleton(&self) -> String {
        self.r#type.base.html_skeleton()
    }

    fn js_load_fn(&self, writer: &mut std::io::BufWriter<std::fs::File>) -> Result<(), GameBuildError> {
        if !self.r#type.base_was_built.load(Ordering::Relaxed) {
            self.r#type.base_was_built.store(true, Ordering::Relaxed);
            self.r#type.base.js_load_fn(writer)
        } else {
            Ok(())
        }
    }

    fn js_load_call(&self) -> String {
        let input_inits = indent_by(8, &self.data.inits);
        let base_call = self.r#type.base.js_load_call();

        indoc::formatdoc! {r#"
            {{
                const outer_inputs = inputs;

                {{
                    let inputs = Object.create(outer_inputs);
                    {input_inits}

                    unloads.push({base_call});
                }}
            }}
        "#}.trim_end().to_string()
    }
}

#[derive(Debug)]
/// Functionally equivalent to inputs, but implements `PieceData`
pub struct CustomPieceData {
    inits: String,
    bindings: String,
}

impl PieceData for CustomPieceData {
    fn js_register(&self) -> Result<(String, String), GameReadError> {
        Ok((String::new(), self.bindings.clone()))
    }
}

impl CustomPieceData {
    fn new(inputs: HashMap<String, DynamicString>) -> Result<Self, GameReadError> {
        let mut inits = Vec::new();
        let mut bindings = HashMap::new();

        for (name, value) in inputs.iter() {
            match value {
                DynamicString::Callback(dsc) => {
                    inits.push(format!("{};", dsc.compile(&format!("inputs.{name}"))?));
                },
                DynamicString::Bind(dsc) => {
                    compile_bindings(dsc, &mut inits, SetData::Variable(format!("inputs.{name}")), &mut bindings)?;
                },
            }
        }

        let data_subscriptions = register_subscriptions(bindings);

        Ok(Self {
            inits: inits.join("\n"),
            bindings: data_subscriptions.join("\n\n"),
        })
    }
}
