//! The method table: host methods carry their schema description, mutation flag and handler, and
//! every module action resolves to a generated `edit.<action>` method from the same registry, so
//! discovery, event emission and dispatch cannot drift apart.
use super::{ApiRequest, ApiResponse, ClientSession, PROTOCOL};
use crate::{
    ActionDescriptor, AssetId, EditorService, EntryId, Error, ErrorKind, HistorySelection,
    ModuleRegistry, Mutation, Zoom,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use std::path::PathBuf;

pub(super) type Handler =
    fn(&mut EditorService, &mut ClientSession, &Value) -> Result<Value, Error>;

pub(super) struct MethodSpec {
    pub name: &'static str,
    pub mutates: bool,
    pub required: &'static [&'static str],
    pub optional: &'static [(&'static str, &'static str)],
    pub notes: &'static str,
    /// `None` marks a method the owner loop answers from its own state.
    pub handler: Option<Handler>,
}

pub(super) const METHODS: &[MethodSpec] = &[
    MethodSpec {
        name: "schema.list",
        mutates: false,
        required: &[],
        optional: &[],
        notes: "protocol identity and every method with its parameters",
        handler: Some(schema_list),
    },
    MethodSpec {
        name: "catalog.import",
        mutates: true,
        required: &["path"],
        optional: &[],
        notes: "references an existing JPEG without copying it and returns the asset state",
        handler: Some(catalog_import),
    },
    MethodSpec {
        name: "catalog.list",
        mutates: false,
        required: &[],
        optional: &[],
        notes: "referenced assets in import order",
        handler: Some(catalog_list),
    },
    MethodSpec {
        name: "asset.state",
        mutates: false,
        required: &["asset_id"],
        optional: &[],
        notes: "current entry, revision and redo path",
        handler: Some(asset_state),
    },
    MethodSpec {
        name: "history.list",
        mutates: false,
        required: &["asset_id"],
        optional: &[("before_sequence", "u64"), ("limit", "1..100")],
        notes: "chronological entries newest first, including abandoned branches",
        handler: Some(history_list),
    },
    MethodSpec {
        name: "history.inspect",
        mutates: false,
        required: &["asset_id", "entry_id"],
        optional: &[],
        notes: "one entry with its complete immutable stack",
        handler: Some(history_inspect),
    },
    MethodSpec {
        name: "history.lineage",
        mutates: false,
        required: &["asset_id"],
        optional: &[
            ("entry_id", "start entry; default current"),
            ("limit", "1..100"),
        ],
        notes: "undo-parent chain newest first; next_entry_id continues a longer chain",
        handler: Some(history_lineage),
    },
    MethodSpec {
        name: "module.list",
        mutates: false,
        required: &[],
        optional: &[],
        notes: "every registered module descriptor with its effects, actions, parameters and controls",
        handler: Some(module_list),
    },
    MethodSpec {
        name: "history.undo",
        mutates: true,
        required: &["asset_id", "mutation"],
        optional: &[],
        notes: "moves current to its undo parent without adding an entry",
        handler: Some(history_undo),
    },
    MethodSpec {
        name: "history.redo",
        mutates: true,
        required: &["asset_id", "mutation"],
        optional: &[],
        notes: "follows the persisted redo path",
        handler: Some(history_redo),
    },
    MethodSpec {
        name: "history.restore",
        mutates: true,
        required: &["asset_id", "mutation", "entry_id"],
        optional: &[],
        notes: "appends a restore action copying the entry's stack and returns the session to current",
        handler: Some(history_restore),
    },
    MethodSpec {
        name: "version.create",
        mutates: true,
        required: &["asset_id", "name", "actor"],
        optional: &[("entry_id", "entry to name; default current")],
        notes: "names a retained entry; unique per asset ignoring case; no-op when the name already names that entry",
        handler: Some(version_create),
    },
    MethodSpec {
        name: "version.delete",
        mutates: true,
        required: &["asset_id", "name"],
        optional: &[],
        notes: "removes the name only; the entry stays in history; no-op when absent",
        handler: Some(version_delete),
    },
    MethodSpec {
        name: "version.list",
        mutates: false,
        required: &["asset_id"],
        optional: &[],
        notes: "saved versions in creation order with their entry sequence",
        handler: Some(version_list),
    },
    MethodSpec {
        name: "preview.select",
        mutates: false,
        required: &["asset_id", "entry_id"],
        optional: &[],
        notes: "read-only session selection; returns generation and session",
        handler: Some(preview_select),
    },
    MethodSpec {
        name: "preview.return-current",
        mutates: false,
        required: &[],
        optional: &[],
        notes: "returns generation and session",
        handler: Some(preview_return_current),
    },
    MethodSpec {
        name: "view.set",
        mutates: false,
        required: &[],
        optional: &[
            ("zoom", "{mode:fit} or {mode:percent,value:10..1600}"),
            ("pan_x", "finite"),
            ("pan_y", "finite"),
        ],
        notes: "session view state; returns the session",
        handler: Some(view_set),
    },
    MethodSpec {
        name: "session.state",
        mutates: false,
        required: &[],
        optional: &[],
        notes: "this client's selection, view and session revision",
        handler: Some(session_state),
    },
    MethodSpec {
        name: "render.sample",
        mutates: false,
        required: &["asset_id", "x", "y"],
        optional: &[],
        notes: "one pixel of the session's selected entry, evaluated without rasterizing",
        handler: Some(render_sample),
    },
    MethodSpec {
        name: "events.since",
        mutates: false,
        required: &["after"],
        optional: &[],
        notes: "gap=true requires an asset.state refresh",
        handler: None,
    },
];

/// A resolved method: a host method from the static table, or one generated from a registered
/// module action. Both come from the same lookup discovery uses.
pub(super) enum Method {
    Host(&'static MethodSpec),
    Action(String),
}

impl Method {
    pub(super) fn mutates(&self) -> bool {
        match self {
            Self::Host(spec) => spec.mutates,
            Self::Action(_) => true,
        }
    }
    /// `true` for the methods the owner loop answers from its own state.
    pub(super) fn owner_answered(&self) -> bool {
        matches!(self, Self::Host(spec) if spec.handler.is_none())
    }
}

/// Action method names are generated: action `set-pixel` is `edit.set-pixel`.
pub(super) fn action_method(action_id: &str) -> String {
    format!("edit.{action_id}")
}

pub(super) fn find(service: &EditorService, name: &str) -> Option<Method> {
    if let Some(spec) = METHODS.iter().find(|spec| spec.name == name) {
        return Some(Method::Host(spec));
    }
    let action_id = name.strip_prefix("edit.")?;
    service
        .registry()
        .action(action_id)
        .map(|_| Method::Action(action_id.to_owned()))
}

/// A method emits an event when it is mutating and its result is not a no-op.
pub(super) fn mutates(method: &Method, result: Option<&Value>) -> bool {
    method.mutates()
        && result
            .and_then(|value| value.get("outcome"))
            .and_then(Value::as_str)
            != Some("no-op")
}

pub(super) fn dispatch(
    service: &mut EditorService,
    session: &mut ClientSession,
    request: &ApiRequest,
    sequence: u64,
) -> ApiResponse {
    let result = match find(service, &request.method) {
        Some(Method::Host(MethodSpec {
            handler: Some(handler),
            ..
        })) => handler(service, session, &request.params),
        Some(Method::Host(_)) => Err(Error::new(
            ErrorKind::Protocol,
            format!("{} is answered by the catalog owner", request.method),
        )),
        Some(Method::Action(action_id)) => {
            edit_action(service, session, &action_id, &request.params)
        }
        None => Err(Error::new(
            ErrorKind::Protocol,
            format!("unknown method {}", request.method),
        )),
    };
    match result {
        Ok(result) => ApiResponse::success(request.id.clone(), sequence, result),
        Err(error) => ApiResponse::failure(request.id.clone(), sequence, error),
    }
}

/// One generated method description: the envelope every action shares plus the action's own
/// declared parameters, so a client needs no hand-maintained list.
fn action_schema(action: &ActionDescriptor) -> Value {
    let mut required = vec![json!("asset_id"), json!("mutation")];
    let mut optional = Map::new();
    for parameter in &action.parameters {
        if parameter.required && parameter.default.is_none() {
            required.push(json!(parameter.name));
        } else {
            optional.insert(parameter.name.clone(), json!(parameter.notes));
        }
    }
    json!({
        "mutates": true,
        "required": required,
        "optional": optional,
        "notes": action.notes,
        "parameters": action.parameters,
    })
}

pub fn schemas(registry: &ModuleRegistry) -> Value {
    let mut methods: Map<String, Value> = METHODS
        .iter()
        .map(|spec| {
            let optional: Map<String, Value> = spec
                .optional
                .iter()
                .map(|(name, meaning)| ((*name).to_string(), json!(meaning)))
                .collect();
            (
                spec.name.to_string(),
                json!({
                    "mutates": spec.mutates,
                    "required": spec.required,
                    "optional": optional,
                    "notes": spec.notes,
                }),
            )
        })
        .collect();
    let descriptors = registry.descriptors();
    for descriptor in &descriptors {
        for action in &descriptor.actions {
            methods.insert(action_method(&action.id), action_schema(action));
        }
    }
    json!({
        "protocol": PROTOCOL,
        "coordinate_space": "Each edit uses integer coordinates in its input image stage after EXIF orientation.",
        "methods": methods,
        "modules": descriptors,
        "mutation": {"required": ["expected_revision", "request_id", "actor"]},
    })
}

fn schema_list(
    service: &mut EditorService,
    _: &mut ClientSession,
    _: &Value,
) -> Result<Value, Error> {
    Ok(schemas(service.registry()))
}

fn module_list(
    service: &mut EditorService,
    _: &mut ClientSession,
    _: &Value,
) -> Result<Value, Error> {
    Ok(json!({"modules": service.registry().descriptors()}))
}

fn catalog_import(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        path: PathBuf,
    }
    value(service.import(&parse::<P>(params)?.path)?)
}

fn catalog_list(
    service: &mut EditorService,
    _: &mut ClientSession,
    _: &Value,
) -> Result<Value, Error> {
    Ok(json!({"assets": service.assets()?}))
}

fn asset_state(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    let p = parse::<AssetParams>(params)?;
    value(service.state(&p.asset_id)?)
}

fn history_list(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        asset_id: AssetId,
        before_sequence: Option<u64>,
        limit: Option<usize>,
    }
    let p = parse::<P>(params)?;
    value(service.history(&p.asset_id, p.before_sequence, p.limit.unwrap_or(50))?)
}

fn history_inspect(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    let p = parse::<EntryParams>(params)?;
    value(service.entry(&p.asset_id, &p.entry_id)?)
}

fn history_lineage(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        asset_id: AssetId,
        entry_id: Option<EntryId>,
        limit: Option<usize>,
    }
    let p = parse::<P>(params)?;
    value(service.lineage(&p.asset_id, p.entry_id.as_ref(), p.limit.unwrap_or(50))?)
}

/// Every generated action method: `asset_id` and `mutation` are the envelope, the remaining
/// top-level fields are the action's declared parameters.
fn edit_action(
    service: &mut EditorService,
    session: &mut ClientSession,
    action_id: &str,
    params: &Value,
) -> Result<Value, Error> {
    require_current(session)?;
    let mut parameters = match params {
        Value::Object(object) => object.clone(),
        Value::Null => Map::new(),
        _ => {
            return Err(Error::new(
                ErrorKind::Validation,
                "params must be a JSON object",
            ));
        }
    };
    let asset_id: AssetId = envelope(&mut parameters, "asset_id")?;
    let mutation: Mutation = envelope(&mut parameters, "mutation")?;
    value(service.apply_action(&asset_id, mutation, action_id, Value::Object(parameters))?)
}

fn envelope<T: DeserializeOwned>(
    parameters: &mut Map<String, Value>,
    name: &str,
) -> Result<T, Error> {
    let field = parameters.remove(name).ok_or_else(|| {
        Error::new(
            ErrorKind::Validation,
            format!("missing required field {name}"),
        )
    })?;
    params(&field)
}

fn history_undo(
    service: &mut EditorService,
    session: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    require_current(session)?;
    let p = parse::<MutationParams>(params)?;
    value(service.undo(&p.asset_id, p.mutation)?)
}

fn history_redo(
    service: &mut EditorService,
    session: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    require_current(session)?;
    let p = parse::<MutationParams>(params)?;
    value(service.redo(&p.asset_id, p.mutation)?)
}

fn history_restore(
    service: &mut EditorService,
    session: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        asset_id: AssetId,
        mutation: Mutation,
        entry_id: EntryId,
    }
    let p = parse::<P>(params)?;
    let result = service.restore(&p.asset_id, p.mutation, &p.entry_id)?;
    if !session.preview.can_edit() {
        session.preview.return_current();
        session.touch();
    }
    value(result)
}

fn version_create(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        asset_id: AssetId,
        name: String,
        actor: String,
        entry_id: Option<EntryId>,
    }
    let p = parse::<P>(params)?;
    value(service.create_version(&p.asset_id, &p.name, p.entry_id.as_ref(), &p.actor)?)
}

fn version_delete(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        asset_id: AssetId,
        name: String,
    }
    let p = parse::<P>(params)?;
    value(service.delete_version(&p.asset_id, &p.name)?)
}

fn version_list(
    service: &mut EditorService,
    _: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    let p = parse::<AssetParams>(params)?;
    Ok(json!({"versions": service.versions(&p.asset_id)?}))
}

fn preview_select(
    service: &mut EditorService,
    session: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    let p = parse::<EntryParams>(params)?;
    service.entry(&p.asset_id, &p.entry_id)?;
    let generation = session.preview.select(HistorySelection::Entry(p.entry_id));
    session.touch();
    Ok(json!({"generation": generation, "session": session}))
}

fn preview_return_current(
    _: &mut EditorService,
    session: &mut ClientSession,
    _: &Value,
) -> Result<Value, Error> {
    let generation = session.preview.return_current();
    session.touch();
    Ok(json!({"generation": generation, "session": session}))
}

fn view_set(
    _: &mut EditorService,
    session: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        zoom: Option<Zoom>,
        pan_x: Option<f32>,
        pan_y: Option<f32>,
    }
    let p = parse::<P>(params)?;
    if let Some(zoom) = p.zoom {
        session.preview.view.set_zoom(zoom)?;
    }
    if p.pan_x.is_some() || p.pan_y.is_some() {
        session.preview.view.pan_to(
            p.pan_x.unwrap_or(session.preview.view.pan_x),
            p.pan_y.unwrap_or(session.preview.view.pan_y),
        )?;
    }
    session.touch();
    value(session)
}

fn session_state(
    _: &mut EditorService,
    session: &mut ClientSession,
    _: &Value,
) -> Result<Value, Error> {
    value(session)
}

fn render_sample(
    service: &mut EditorService,
    session: &mut ClientSession,
    params: &Value,
) -> Result<Value, Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct P {
        asset_id: AssetId,
        x: u32,
        y: u32,
    }
    let p = parse::<P>(params)?;
    let entry_id = match &session.preview.selection {
        HistorySelection::Current => service.state(&p.asset_id)?.current_entry.id,
        HistorySelection::Entry(id) => id.clone(),
    };
    let mut sampled = value(service.sample_entry(&p.asset_id, &entry_id, p.x, p.y)?)?;
    sampled["source_detail_ready"] = json!(true);
    Ok(sampled)
}

fn require_current(session: &ClientSession) -> Result<(), Error> {
    if session.preview.can_edit() {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::Conflict,
            "return to current or restore the selected history entry before editing",
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetParams {
    asset_id: AssetId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryParams {
    asset_id: AssetId,
    entry_id: EntryId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationParams {
    asset_id: AssetId,
    mutation: Mutation,
}

fn parse<T: DeserializeOwned>(value: &Value) -> Result<T, Error> {
    params(value)
}
pub(super) fn params<T: DeserializeOwned>(value: &Value) -> Result<T, Error> {
    serde_json::from_value(value.clone())
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))
}
fn value(value: impl Serialize) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|error| Error::new(ErrorKind::Internal, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn host_and_generated_methods_are_unique_complete_and_match_the_schema() {
        let catalog =
            std::env::temp_dir().join(format!("lightwell-methods-{}.sqlite", std::process::id()));
        let mut service = EditorService::open(&catalog).unwrap();
        let mut session = ClientSession::default();
        let names: HashSet<&str> = METHODS.iter().map(|spec| spec.name).collect();
        assert_eq!(names.len(), METHODS.len(), "duplicate method names");
        let generated: Vec<String> = service
            .registry()
            .descriptors()
            .iter()
            .flat_map(|descriptor| descriptor.actions.iter())
            .map(|action| action_method(&action.id))
            .collect();
        assert_eq!(generated, ["edit.set-pixel", "edit.transform"]);
        let schema = schemas(service.registry());
        let listed = schema["methods"].as_object().unwrap();
        assert_eq!(listed.len(), METHODS.len() + generated.len());
        assert_eq!(
            schema["modules"].as_array().unwrap().len(),
            service.registry().descriptors().len()
        );
        assert_eq!(
            listed["edit.set-pixel"]["required"],
            json!(["asset_id", "mutation", "x", "y", "rgb"])
        );
        assert_eq!(
            listed["edit.set-pixel"]["parameters"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(listed["edit.transform"]["mutates"], json!(true));
        for name in names
            .iter()
            .copied()
            .chain(generated.iter().map(String::as_str))
        {
            let spec = find(&service, name).expect("listed methods resolve");
            let request = ApiRequest {
                id: "schema".into(),
                method: name.into(),
                params: json!({}),
                token: None,
            };
            let response = dispatch(&mut service, &mut session, &request, 0);
            if let Some(error) = &response.error {
                assert!(
                    !error.message.starts_with("unknown method"),
                    "{name} is listed but not dispatched"
                );
                if spec.owner_answered() {
                    assert_eq!(error.code, "protocol");
                }
            }
            assert!(
                !mutates(&spec, Some(&json!({"outcome": "no-op"}))),
                "{name} must not emit events for a no-op"
            );
            assert_eq!(
                mutates(&spec, Some(&json!({"outcome": "applied"}))),
                spec.mutates()
            );
            assert!(!listed[name]["notes"].as_str().unwrap().is_empty());
        }
        assert!(find(&service, "edit.missing").is_none());
        assert!(find(&service, "set-pixel").is_none());
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }
}
