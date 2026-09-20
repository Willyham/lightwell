//! Plain-data module descriptors: one serializable source for API discovery, generated controls
//! and every validation limit a module declares.
use crate::{Error, ErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;

/// Groups nest for layout only; a descriptor deeper than this is rejected rather than walked.
const MAX_CONTROL_DEPTH: usize = 8;

fn validation(detail: impl Into<String>) -> Error {
    Error::new(ErrorKind::Validation, detail)
}

fn valid_segments(value: &str, separator: char, alphabetic_segments: bool) -> bool {
    !value.is_empty()
        && value.split(separator).enumerate().all(|(index, segment)| {
            let mut bytes = segment.bytes();
            let Some(first) = bytes.next() else {
                return false;
            };
            let starts = first.is_ascii_lowercase()
                || (!alphabetic_segments && index > 0 && first.is_ascii_digit());
            starts && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

/// Module and effect identity: lowercase words separated by dots, e.g. `lightwell.pixel.replace`.
pub fn valid_identity(value: &str) -> bool {
    valid_segments(value, '.', true)
}

/// Action and parameter identity: lowercase words separated by hyphens, e.g. `set-pixel`.
pub fn valid_name(value: &str) -> bool {
    valid_segments(value, '-', false)
}

/// Where an effect acts: geometry changes the stage, pixel effects address their input stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectStage {
    Geometry,
    Pixel,
}

/// A durable effect identity stored in every layer, with its internal payload format marker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectDescriptor {
    pub id: String,
    pub format: u32,
    pub stage: EffectStage,
}

/// The closed set of parameter types v0 modules may declare.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ParameterKind {
    Integer {
        min: i64,
        max: i64,
    },
    Enum {
        options: Vec<String>,
    },
    /// Three 8-bit sRGB channels as a JSON array.
    Color,
}

/// Serialized flat: `{"name": "x", "kind": "integer", "min": 0, "max": 16383, ...}`. Flattening
/// the kind rules out `deny_unknown_fields` here; unknown fields are ignored on read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterDescriptor {
    pub name: String,
    #[serde(flatten)]
    pub kind: ParameterKind,
    pub required: bool,
    pub default: Option<Value>,
    pub unit: Option<String>,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDescriptor {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub parameters: Vec<ParameterDescriptor>,
}

impl ActionDescriptor {
    pub fn parameter(&self, name: &str) -> Option<&ParameterDescriptor> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name == name)
    }
}

/// Ordered semantic controls. A client renders them; it never invents an operation of its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Control {
    Group {
        label: String,
        controls: Vec<Control>,
    },
    Number {
        action: String,
        parameter: String,
        label: String,
    },
    Color {
        action: String,
        parameter: String,
        label: String,
    },
    Action {
        action: String,
        label: String,
        #[serde(default)]
        preset: Map<String, Value>,
    },
}

/// A pointer pick on the image fills declared integer parameters; it never commits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CanvasInteraction {
    PointPick {
        action: String,
        x: String,
        y: String,
    },
}

/// An unavailable provider keeps its descriptor and effect identities so stored data stays readable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Availability {
    Available,
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleDescriptor {
    pub id: String,
    pub title: String,
    pub effects: Vec<EffectDescriptor>,
    pub actions: Vec<ActionDescriptor>,
    pub controls: Vec<Control>,
    pub canvas: Option<CanvasInteraction>,
    pub availability: Availability,
}

impl ModuleDescriptor {
    /// Read a descriptor from JSON, reporting a missing or malformed field as a validation error.
    pub fn parse(value: &Value) -> Result<Self, Error> {
        let descriptor: Self = serde_json::from_value(value.clone())
            .map_err(|error| validation(format!("invalid module descriptor: {error}")))?;
        descriptor.validate()?;
        Ok(descriptor)
    }

    pub fn action(&self, id: &str) -> Option<&ActionDescriptor> {
        self.actions.iter().find(|action| action.id == id)
    }

    pub fn effect(&self, id: &str) -> Option<&EffectDescriptor> {
        self.effects.iter().find(|effect| effect.id == id)
    }

    pub fn is_available(&self) -> bool {
        matches!(self.availability, Availability::Available)
    }

    /// Reject every descriptor a client could not render or validate against.
    pub fn validate(&self) -> Result<(), Error> {
        if !valid_identity(&self.id) {
            return Err(validation(format!("invalid module identity {}", self.id)));
        }
        if self.title.trim().is_empty() {
            return Err(validation(format!("module {} has no title", self.id)));
        }
        let mut effects = HashSet::with_capacity(self.effects.len());
        for effect in &self.effects {
            if !valid_identity(&effect.id) {
                return Err(validation(format!("invalid effect identity {}", effect.id)));
            }
            if !effects.insert(effect.id.as_str()) {
                return Err(validation(format!("duplicate effect {}", effect.id)));
            }
        }
        let mut actions = HashSet::with_capacity(self.actions.len());
        for action in &self.actions {
            if !valid_name(&action.id) {
                return Err(validation(format!("invalid action identity {}", action.id)));
            }
            if !actions.insert(action.id.as_str()) {
                return Err(validation(format!("duplicate action {}", action.id)));
            }
            if action.title.trim().is_empty() {
                return Err(validation(format!("action {} has no title", action.id)));
            }
            let mut parameters = HashSet::with_capacity(action.parameters.len());
            for parameter in &action.parameters {
                if !valid_name(&parameter.name) {
                    return Err(validation(format!(
                        "invalid parameter name {} of action {}",
                        parameter.name, action.id
                    )));
                }
                if !parameters.insert(parameter.name.as_str()) {
                    return Err(validation(format!(
                        "duplicate parameter {} of action {}",
                        parameter.name, action.id
                    )));
                }
                match &parameter.kind {
                    ParameterKind::Integer { min, max } if min > max => {
                        return Err(validation(format!(
                            "parameter {} declares an empty range {min}..={max}",
                            parameter.name
                        )));
                    }
                    ParameterKind::Enum { options } if options.is_empty() => {
                        return Err(validation(format!(
                            "parameter {} declares no options",
                            parameter.name
                        )));
                    }
                    _ => {}
                }
                if let Some(default) = &parameter.default {
                    check_value(parameter, default)?;
                }
            }
        }
        for control in &self.controls {
            self.check_control(control, 1)?;
        }
        match &self.canvas {
            Some(CanvasInteraction::PointPick { action, x, y }) => {
                let declared = self.declared_action(action)?;
                for name in [x, y] {
                    let parameter = self.declared_parameter(declared, name)?;
                    if !matches!(parameter.kind, ParameterKind::Integer { .. }) {
                        return Err(validation(format!(
                            "canvas parameter {name} of action {action} is not an integer"
                        )));
                    }
                }
            }
            None => {}
        }
        Ok(())
    }

    fn declared_action(&self, id: &str) -> Result<&ActionDescriptor, Error> {
        self.action(id).ok_or_else(|| {
            validation(format!(
                "module {} references undeclared action {id}",
                self.id
            ))
        })
    }

    fn declared_parameter<'a>(
        &self,
        action: &'a ActionDescriptor,
        name: &str,
    ) -> Result<&'a ParameterDescriptor, Error> {
        action
            .parameter(name)
            .ok_or_else(|| validation(format!("action {} has no parameter {name}", action.id)))
    }

    fn check_control(&self, control: &Control, depth: usize) -> Result<(), Error> {
        if depth > MAX_CONTROL_DEPTH {
            return Err(validation(format!(
                "module {} nests controls deeper than {MAX_CONTROL_DEPTH} levels",
                self.id
            )));
        }
        match control {
            Control::Group { label, controls } => {
                if label.trim().is_empty() {
                    return Err(validation(format!(
                        "module {} has an unlabelled group",
                        self.id
                    )));
                }
                for child in controls {
                    self.check_control(child, depth + 1)?;
                }
            }
            Control::Number {
                action, parameter, ..
            } => {
                let declared = self.declared_action(action)?;
                let declared = self.declared_parameter(declared, parameter)?;
                if !matches!(declared.kind, ParameterKind::Integer { .. }) {
                    return Err(validation(format!(
                        "number control for {parameter} of action {action} is not an integer"
                    )));
                }
            }
            Control::Color {
                action, parameter, ..
            } => {
                let declared = self.declared_action(action)?;
                let declared = self.declared_parameter(declared, parameter)?;
                if !matches!(declared.kind, ParameterKind::Color) {
                    return Err(validation(format!(
                        "color control for {parameter} of action {action} is not a color"
                    )));
                }
            }
            Control::Action { action, preset, .. } => {
                let declared = self.declared_action(action)?;
                for (name, value) in preset {
                    check_value(self.declared_parameter(declared, name)?, value)?;
                }
            }
        }
        Ok(())
    }
}

fn check_value(parameter: &ParameterDescriptor, value: &Value) -> Result<(), Error> {
    let name = &parameter.name;
    match &parameter.kind {
        ParameterKind::Integer { min, max } => {
            let number = value
                .as_i64()
                .ok_or_else(|| validation(format!("parameter {name} must be an integer")))?;
            if number < *min || number > *max {
                return Err(validation(format!(
                    "parameter {name} must be an integer within {min}..={max}"
                )));
            }
        }
        ParameterKind::Enum { options } => {
            let text = value
                .as_str()
                .ok_or_else(|| validation(format!("parameter {name} must be a string")))?;
            if !options.iter().any(|option| option == text) {
                return Err(validation(format!(
                    "parameter {name} must be one of {}",
                    options.join(", ")
                )));
            }
        }
        ParameterKind::Color => {
            let valid = value.as_array().is_some_and(|channels| {
                channels.len() == 3
                    && channels
                        .iter()
                        .all(|channel| channel.as_u64().is_some_and(|channel| channel <= 255))
            });
            if !valid {
                return Err(validation(format!(
                    "parameter {name} must be three sRGB channels 0..=255"
                )));
            }
        }
    }
    Ok(())
}

/// Apply declared defaults and reject anything an action did not declare, so every caller of an
/// action gets the same structured validation error before the module sees the request.
pub fn check_parameters(
    action: &ActionDescriptor,
    input: &Value,
) -> Result<Map<String, Value>, Error> {
    let empty = Map::new();
    let object = match input {
        Value::Object(object) => object,
        Value::Null => &empty,
        _ => {
            return Err(validation(format!(
                "parameters of action {} must be a JSON object",
                action.id
            )));
        }
    };
    for name in object.keys() {
        if action.parameter(name).is_none() {
            return Err(validation(format!(
                "unknown parameter {name} for action {}",
                action.id
            )));
        }
    }
    let mut checked = Map::new();
    for parameter in &action.parameters {
        match (object.get(&parameter.name), &parameter.default) {
            (Some(value), _) => {
                check_value(parameter, value)?;
                checked.insert(parameter.name.clone(), value.clone());
            }
            (None, Some(default)) => {
                checked.insert(parameter.name.clone(), default.clone());
            }
            (None, None) if parameter.required => {
                return Err(validation(format!(
                    "missing required parameter {} for action {}",
                    parameter.name, action.id
                )));
            }
            (None, None) => {}
        }
    }
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn integer(name: &str) -> ParameterDescriptor {
        ParameterDescriptor {
            name: name.into(),
            kind: ParameterKind::Integer { min: 0, max: 10 },
            required: true,
            default: None,
            unit: Some("px".into()),
            notes: "test".into(),
        }
    }

    fn action() -> ActionDescriptor {
        ActionDescriptor {
            id: "set-thing".into(),
            title: "Set thing".into(),
            notes: "test".into(),
            parameters: vec![
                integer("x"),
                ParameterDescriptor {
                    name: "rgb".into(),
                    kind: ParameterKind::Color,
                    required: true,
                    default: None,
                    unit: None,
                    notes: "test".into(),
                },
                ParameterDescriptor {
                    name: "mode".into(),
                    kind: ParameterKind::Enum {
                        options: vec!["fast".into(), "exact".into()],
                    },
                    required: false,
                    default: Some(json!("exact")),
                    unit: None,
                    notes: "test".into(),
                },
            ],
        }
    }

    fn descriptor() -> ModuleDescriptor {
        ModuleDescriptor {
            id: "test.module".into(),
            title: "Test".into(),
            effects: vec![EffectDescriptor {
                id: "test.module.effect".into(),
                format: 1,
                stage: EffectStage::Pixel,
            }],
            actions: vec![action()],
            controls: vec![Control::Group {
                label: "Test".into(),
                controls: vec![
                    Control::Number {
                        action: "set-thing".into(),
                        parameter: "x".into(),
                        label: "X".into(),
                    },
                    Control::Action {
                        action: "set-thing".into(),
                        label: "Apply".into(),
                        preset: Map::new(),
                    },
                ],
            }],
            canvas: None,
            availability: Availability::Available,
        }
    }

    #[test]
    fn identity_rules_accept_declared_names_and_reject_malformed_ones() {
        for value in ["lightwell.pixel", "a", "lightwell.pixel.replace", "a1.b2"] {
            assert!(valid_identity(value), "{value}");
        }
        for value in [
            "",
            "Lightwell.pixel",
            "lightwell..pixel",
            ".pixel",
            "pixel.",
            "light_well",
            "1pixel",
            "set-pixel",
        ] {
            assert!(!valid_identity(value), "{value}");
        }
        for value in ["set-pixel", "transform", "rotate-left", "x", "crop-16-9"] {
            assert!(valid_name(value), "{value}");
        }
        for value in [
            "",
            "Set-Pixel",
            "set--pixel",
            "-set",
            "set-",
            "set.pixel",
            "1set",
        ] {
            assert!(!valid_name(value), "{value}");
        }
    }

    #[test]
    fn descriptors_reject_malformed_identities_duplicates_and_invalid_controls() {
        assert!(descriptor().validate().is_ok());
        let cases: Vec<(&str, ModuleDescriptor)> = vec![
            (
                "module identity",
                ModuleDescriptor {
                    id: "Test Module".into(),
                    ..descriptor()
                },
            ),
            (
                "module title",
                ModuleDescriptor {
                    title: "  ".into(),
                    ..descriptor()
                },
            ),
            (
                "effect identity",
                ModuleDescriptor {
                    effects: vec![EffectDescriptor {
                        id: "Test-Effect".into(),
                        format: 1,
                        stage: EffectStage::Pixel,
                    }],
                    ..descriptor()
                },
            ),
            (
                "duplicate effect",
                ModuleDescriptor {
                    effects: vec![
                        descriptor().effects[0].clone(),
                        descriptor().effects[0].clone(),
                    ],
                    ..descriptor()
                },
            ),
            (
                "duplicate action",
                ModuleDescriptor {
                    actions: vec![action(), action()],
                    ..descriptor()
                },
            ),
            (
                "action identity",
                ModuleDescriptor {
                    actions: vec![ActionDescriptor {
                        id: "Set.Thing".into(),
                        ..action()
                    }],
                    controls: Vec::new(),
                    ..descriptor()
                },
            ),
            (
                "duplicate parameter",
                ModuleDescriptor {
                    actions: vec![ActionDescriptor {
                        parameters: vec![integer("x"), integer("x")],
                        ..action()
                    }],
                    controls: Vec::new(),
                    ..descriptor()
                },
            ),
            (
                "empty integer range",
                ModuleDescriptor {
                    actions: vec![ActionDescriptor {
                        parameters: vec![ParameterDescriptor {
                            kind: ParameterKind::Integer { min: 5, max: 1 },
                            ..integer("x")
                        }],
                        ..action()
                    }],
                    controls: Vec::new(),
                    ..descriptor()
                },
            ),
            (
                "empty enum",
                ModuleDescriptor {
                    actions: vec![ActionDescriptor {
                        parameters: vec![ParameterDescriptor {
                            name: "mode".into(),
                            kind: ParameterKind::Enum {
                                options: Vec::new(),
                            },
                            required: true,
                            default: None,
                            unit: None,
                            notes: "test".into(),
                        }],
                        ..action()
                    }],
                    controls: Vec::new(),
                    ..descriptor()
                },
            ),
            (
                "default out of range",
                ModuleDescriptor {
                    actions: vec![ActionDescriptor {
                        parameters: vec![ParameterDescriptor {
                            default: Some(json!(99)),
                            ..integer("x")
                        }],
                        ..action()
                    }],
                    controls: Vec::new(),
                    ..descriptor()
                },
            ),
            (
                "undeclared action",
                ModuleDescriptor {
                    controls: vec![Control::Number {
                        action: "missing".into(),
                        parameter: "x".into(),
                        label: "X".into(),
                    }],
                    ..descriptor()
                },
            ),
            (
                "undeclared parameter",
                ModuleDescriptor {
                    controls: vec![Control::Number {
                        action: "set-thing".into(),
                        parameter: "missing".into(),
                        label: "X".into(),
                    }],
                    ..descriptor()
                },
            ),
            (
                "wrong control kind",
                ModuleDescriptor {
                    controls: vec![Control::Color {
                        action: "set-thing".into(),
                        parameter: "x".into(),
                        label: "X".into(),
                    }],
                    ..descriptor()
                },
            ),
            (
                "preset out of range",
                ModuleDescriptor {
                    controls: vec![Control::Action {
                        action: "set-thing".into(),
                        label: "Apply".into(),
                        preset: json!({"x": 99}).as_object().unwrap().clone(),
                    }],
                    ..descriptor()
                },
            ),
            (
                "preset names an undeclared parameter",
                ModuleDescriptor {
                    controls: vec![Control::Action {
                        action: "set-thing".into(),
                        label: "Apply".into(),
                        preset: json!({"missing": 1}).as_object().unwrap().clone(),
                    }],
                    ..descriptor()
                },
            ),
            (
                "canvas parameter is not an integer",
                ModuleDescriptor {
                    canvas: Some(CanvasInteraction::PointPick {
                        action: "set-thing".into(),
                        x: "x".into(),
                        y: "rgb".into(),
                    }),
                    ..descriptor()
                },
            ),
        ];
        for (case, descriptor) in cases {
            let error = descriptor
                .validate()
                .expect_err(&format!("{case} must be rejected"));
            assert_eq!(error.kind, ErrorKind::Validation, "{case}");
        }
    }

    #[test]
    fn a_parameter_without_a_kind_is_a_validation_error() {
        let mut value = serde_json::to_value(descriptor()).unwrap();
        value["actions"][0]["parameters"][0]
            .as_object_mut()
            .unwrap()
            .remove("kind");
        let error = ModuleDescriptor::parse(&value).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Validation);
        assert!(
            error.detail.contains("invalid module descriptor"),
            "{error}"
        );
        assert_eq!(
            ModuleDescriptor::parse(&serde_json::to_value(descriptor()).unwrap()).unwrap(),
            descriptor()
        );
    }

    #[test]
    fn generic_parameter_checks_apply_defaults_and_name_the_broken_parameter() {
        let action = action();
        let checked = check_parameters(&action, &json!({"x": 3, "rgb": [1, 2, 3]})).unwrap();
        assert_eq!(checked["x"], json!(3));
        assert_eq!(checked["mode"], json!("exact"), "declared default applied");
        for (case, input, fragment) in [
            (
                "missing required",
                json!({"x": 1}),
                "missing required parameter rgb",
            ),
            (
                "unknown field",
                json!({"x": 1, "rgb": [1, 2, 3], "z": 4}),
                "unknown parameter z",
            ),
            (
                "out of range",
                json!({"x": 11, "rgb": [1, 2, 3]}),
                "parameter x must be an integer within 0..=10",
            ),
            (
                "not an integer",
                json!({"x": 1.5, "rgb": [1, 2, 3]}),
                "parameter x must be an integer",
            ),
            (
                "unknown enum option",
                json!({"x": 1, "rgb": [1, 2, 3], "mode": "sloppy"}),
                "parameter mode must be one of fast, exact",
            ),
            (
                "malformed color",
                json!({"x": 1, "rgb": [1, 2, 300]}),
                "parameter rgb must be three sRGB channels 0..=255",
            ),
            (
                "short color",
                json!({"x": 1, "rgb": [1, 2]}),
                "parameter rgb must be three sRGB channels 0..=255",
            ),
            ("not an object", json!([1, 2]), "must be a JSON object"),
        ] {
            let error = check_parameters(&action, &input).expect_err(case);
            assert_eq!(error.kind, ErrorKind::Validation, "{case}");
            assert!(error.detail.contains(fragment), "{case}: {error}");
        }
    }
}
