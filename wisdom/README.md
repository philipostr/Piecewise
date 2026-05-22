# Wisdom - Game specification

Wisdom is, in simple terms, the compiler that takes Piecewise game design to a real working static HTML/CSS/JS SPA. The game is built in the Piecewise GUI, but Wisdom is what makes the game become a reality.

## Wisdom YAML schema

TBD

## DSC (Dynamic String Context)

This is the language that a Piecewise game designer uses to implement logic and moving parts in their games. It functions similarly to C# in Unity, GDScript in Godot, Lua in Roblox, etc. It is almost entirely equivalent to JS, but with a few extra pieces of syntax to help the game designer write concise Piecewise-specific code, without the hassle of boilerplate.

### DSC docs

TBD

## Things to improve

* Improve `Piece` trait functions. There's a lot of overlap, maybe a derive macro?
* Support CSS classes and general styling.
* Figure out what piece events would be used for, and implement it.
* Add `vars`: Values that are helpers for the current piece but not accessible by children.
* Add custom piece types with inputs (like custom components in React).
* Protect states and vars with mutex locks
* Improve the efficiency of `Generator` updating (currently just reloads everything always)

### DSC-related stuff
* Document the dynamic string context language.
* Deal with format strings somehow. Currently, any state references to be inline formatted will simply be ignored.
* Maybe support multi-line mutation callbacks?
* Just... ignore comments entirely. Less work to do, and it doesn't matter ultimately anwyay.
* REMOVE THE MANDATORY `bind` AND `callback` HEADERS
