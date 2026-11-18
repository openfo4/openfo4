use openfo4_esp::records::misc_object::MiscObject;
use openfo4_esp::records::{Array, FormList, Keyword};
use openfo4_game::systems::scripting::invocations::Value;

use crate::runner::TestRunner;

#[test]
fn form_get_id() {
    let mut builder = TestRunner::builder();

    let id = builder.add_record(MiscObject::default());

    let mut runner = builder.build();
    let item = runner.load(id);
    let res = runner.call(item, "GetFormID", vec![]);
    assert_eq!(res[0].as_int().unwrap().cast_unsigned(), id.form.0);
}

#[test]
fn form_has_keyword() {
    let mut builder = TestRunner::builder();

    let keyword = builder.add_record(Keyword::default());
    let mut item = MiscObject::default();
    item.keywords = Some(Array(vec![keyword.form]));
    let item_t = builder.add_record(item);
    let item_f = builder.add_record(MiscObject::default());

    let mut runner = builder.build();
    let item_t = runner.load(item_t);
    let item_f = runner.load(item_f);
    let keyword = runner.load(keyword);
    let keyword = runner.script(&keyword, "Keyword");

    let res = runner.call(item_t, "HasKeyword", vec![Value::Object(keyword.clone())]);
    assert_eq!(res[0].as_bool().unwrap(), true);

    let res = runner.call(item_f, "HasKeyword", vec![Value::Object(keyword)]);
    assert_eq!(res[0].as_bool().unwrap(), false);
}

#[test]
fn form_has_keyword_with_null_keyword() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());

    let mut runner = builder.build();
    let item = runner.load(item);

    let res = runner.call(item, "HasKeyword", vec![Value::Null]);
    assert_eq!(res[0].as_bool().unwrap(), false);
}

#[test]
fn form_has_keyword_in_formlist() {
    let mut builder = TestRunner::builder();

    let keyword = builder.add_record(Keyword::default());

    let mut formlist = FormList::default();
    formlist.forms.push(keyword.form);
    let formlist = builder.add_record(formlist);

    let mut item = MiscObject::default();
    item.keywords = Some(Array(vec![keyword.form]));
    let item_t = builder.add_record(item);
    let item_f = builder.add_record(MiscObject::default());

    let mut runner = builder.build();
    let item_t = runner.load(item_t);
    let item_f = runner.load(item_f);
    let formlist = runner.load(formlist);
    let formlist = runner.script(&formlist, "FormList");

    let res = runner.call(
        item_t,
        "HasKeywordInFormList",
        vec![Value::Object(formlist.clone())],
    );
    assert_eq!(res[0].as_bool().unwrap(), true);

    let res = runner.call(
        item_f,
        "HasKeywordInFormList",
        vec![Value::Object(formlist)],
    );
    assert_eq!(res[0].as_bool().unwrap(), false);
}

#[test]
fn form_has_keyword_in_formlist_with_null_formlist() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());

    let mut runner = builder.build();
    let item = runner.load(item);

    let res = runner.call(item, "HasKeywordInFormList", vec![Value::Null]);
    assert_eq!(res[0].as_bool().unwrap(), false);
}
