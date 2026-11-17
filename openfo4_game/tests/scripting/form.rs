use openfo4_esp::records::misc_object::MiscObject;

use crate::runner::TestRunner;

#[test]
fn test_form_get_id() {
    let mut builder = TestRunner::builder();

    let id = builder.add_record(MiscObject::default());

    let mut runner = builder.build();
    let item = runner.load(id);
    let res = runner.call(item, "GetFormID", vec![]);
    assert_eq!(res[0].as_int().unwrap().cast_unsigned(), id.form.0);
}
