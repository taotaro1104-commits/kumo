//! グローバル環境の組み立て：プロトタイプの連鎖、コンストラクタ、`document`、ネイティブ関数、プレリュード。

use boa_engine::object::FunctionObjectBuilder;
use boa_engine::property::{Attribute, PropertyDescriptor};
use boa_engine::{Context, JsObject, JsResult, JsString, JsValue, NativeFunction, Source};
use boa_runtime::extensions::{
    Base64Extension, EncodingExtension, MicrotaskExtension, StructuredCloneExtension, UrlExtension,
};

use crate::host::Host;
use crate::wrap::{type_error, wrap_node};

const PRELUDE: &str = include_str!("prelude.js");

/// `Node.ELEMENT_NODE` などの定数。
const NODE_CONSTANTS: [(&str, i32); 6] = [
    ("ELEMENT_NODE", 1),
    ("TEXT_NODE", 3),
    ("COMMENT_NODE", 8),
    ("DOCUMENT_NODE", 9),
    ("DOCUMENT_TYPE_NODE", 10),
    ("DOCUMENT_FRAGMENT_NODE", 11),
];

fn illegal_constructor(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Err(type_error("Illegal constructor"))
}

/// `new` できない（`instanceof` と継承の起点にだけ使う）コンストラクタを登録する。
fn register_class(ctx: &mut Context, name: &str, proto: &JsObject) -> JsObject {
    let constructor: JsObject = FunctionObjectBuilder::new(
        ctx.realm(),
        NativeFunction::from_fn_ptr(illegal_constructor),
    )
    .name(JsString::from(name))
    .length(0)
    .constructor(true)
    .build()
    .into();
    let prototype = PropertyDescriptor::builder()
        .value(proto.clone())
        .writable(false)
        .enumerable(false);
    let _ = constructor.define_property_or_throw(JsString::from("prototype"), prototype, ctx);
    let _ = proto.set(
        JsString::from("constructor"),
        constructor.clone(),
        false,
        ctx,
    );
    let _ =
        ctx.register_global_property(JsString::from(name), constructor.clone(), Attribute::all());
    constructor
}

fn derived(ctx: &mut Context, parent: &JsObject) -> JsObject {
    let object = JsObject::with_object_proto(ctx.intrinsics());
    let _ = object.set_prototype(Some(parent.clone()));
    object
}

pub(crate) fn bootstrap(ctx: &mut Context, host: &Host) -> JsResult<()> {
    let event_target = JsObject::with_object_proto(ctx.intrinsics());
    let node = derived(ctx, &event_target);
    let character_data = derived(ctx, &node);
    let text = derived(ctx, &character_data);
    let element = derived(ctx, &node);
    let html_element = derived(ctx, &element);
    let document = derived(ctx, &node);
    let fragment = derived(ctx, &node);
    {
        let mut state = host.state_mut();
        for (name, proto) in [
            ("event_target", &event_target),
            ("node", &node),
            ("text", &text),
            ("element", &element),
            ("html_element", &html_element),
            ("document", &document),
            ("fragment", &fragment),
        ] {
            state.protos.insert(name, proto.clone());
        }
    }

    crate::dispatch::install_event_target(ctx, &event_target);
    crate::node_api::install(ctx, &node);
    crate::node_api::install_text(ctx, &character_data);
    crate::element_api::install(ctx, &element, &html_element);
    crate::parent_node::install_parent_node(ctx, &fragment);
    crate::style_api::install(ctx, &element);
    crate::events::install(ctx, &element);
    crate::document_api::install(ctx, &document);

    let node_class = register_class(ctx, "Node", &node);
    for (name, value) in NODE_CONSTANTS {
        let _ = node_class.set(JsString::from(name), value, false, ctx);
    }
    for (name, proto) in [
        ("CharacterData", &character_data),
        ("Text", &text),
        ("Element", &element),
        ("HTMLElement", &html_element),
        ("Document", &document),
        ("DocumentFragment", &fragment),
    ] {
        register_class(ctx, name, proto);
    }

    let global = ctx.global_object();
    crate::dispatch::install_event_target(ctx, &global);
    crate::timers::install(ctx, &global);
    crate::natives::install(ctx, &global);
    crate::computed_style::install(ctx, &global);

    let root = host.doc().borrow().root();
    let document_object = wrap_node(ctx, root);
    ctx.register_global_property(
        JsString::from("document"),
        document_object,
        Attribute::all(),
    )?;

    boa_runtime::register_extensions(
        (
            UrlExtension,
            EncodingExtension,
            Base64Extension,
            StructuredCloneExtension,
            MicrotaskExtension,
        ),
        None,
        ctx,
    )?;
    ctx.eval(Source::from_bytes(PRELUDE))?;
    Ok(())
}
