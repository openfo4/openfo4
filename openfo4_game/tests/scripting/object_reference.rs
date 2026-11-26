use openfo4_esp::records::misc_object::MiscObject;
use openfo4_esp::records::{Array, Keyword, PlacedObject, PositionRotation};
use openfo4_game::systems::scripting::invocations::Value;

use crate::runner::{DefaultExt, TestRunner};

#[test]
fn object_reference_get_base_object() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let item = runner.load(item);
    let item = runner.script(&item, "Form");
    let refr = runner.load(refr);

    runner.call_assert(&refr, "GetBaseObject", [], item);
}

#[test]
fn object_reference_get_position() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
        obj.position = Some(PositionRotation {
            position_x: 1.0,
            position_y: 2.0,
            position_z: 3.0,
            rotation_x: 0.0,
            rotation_y: 0.0,
            rotation_z: 0.0,
        });
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);

    runner.call_assert_approx(&refr, "GetPositionX", [], 1.0);
    runner.call_assert_approx(&refr, "GetPositionY", [], 2.0);
    runner.call_assert_approx(&refr, "GetPositionZ", [], 3.0);
}

#[test]
fn object_reference_get_bounds() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default_with(|item| {
        item.object_bounds.min_x = -1;
        item.object_bounds.max_x = 1;
        item.object_bounds.min_y = -2;
        item.object_bounds.max_y = 2;
        item.object_bounds.min_z = -3;
        item.object_bounds.max_z = 3;
    }));
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);

    runner.call_assert_approx(&refr, "GetWidth", [], 2.0);
    runner.call_assert_approx(&refr, "GetLength", [], 4.0);
    runner.call_assert_approx(&refr, "GetHeight", [], 6.0);
}

#[test]
fn object_reference_get_distance() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());
    let refr1 = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
        obj.position = Some(PositionRotation::default_with(|pos| {
            pos.position_x = -100.0;
        }));
    }));
    let refr2 = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
        obj.position = Some(PositionRotation::default_with(|pos| {
            pos.position_x = 100.0;
        }));
    }));

    let mut runner = builder.build();

    let refr1 = runner.load(refr1);
    let refr2 = runner.load(refr2);

    let other = runner.script(&refr2, "ObjectReference");
    runner.call_assert_approx(&refr1, "GetDistance", [Value::Object(other)], 200.0);

    let other = runner.script(&refr1, "ObjectReference");
    runner.call_assert_approx(&refr2, "GetDistance", [Value::Object(other)], 200.0);
}

#[test]
fn object_reference_has_keyword() {
    let mut builder = TestRunner::builder();

    let keyword = builder.add_record(Keyword::default());

    let item_f = builder.add_record(MiscObject::default());
    let item_t = builder.add_record(MiscObject::default_with(|item| {
        item.keywords = Some(Array(vec![keyword.form]));
    }));

    let refr_f = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item_f.form;
    }));
    let refr_t = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item_t.form;
    }));

    let mut runner = builder.build();

    let keyword = runner.load(keyword);
    let keyword = runner.script(&keyword, "Keyword");

    let refr_f = runner.load(refr_f);
    let refr_t = runner.load(refr_t);

    runner.call_assert(
        &refr_f,
        "HasKeyword",
        [Value::Object(keyword.clone())],
        false,
    );
    runner.call_assert(&refr_t, "HasKeyword", [Value::Object(keyword)], true);
}

#[test]
fn object_reference_has_keyword_null() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);
    runner.call_assert(refr, "HasKeyword", [Value::Null], false);
}

#[test]
fn object_reference_add_keyword() {
    let mut builder = TestRunner::builder();

    let keyword = builder.add_record(Keyword::default());

    let item = builder.add_record(MiscObject::default());
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let keyword = runner.load(keyword);
    let keyword = runner.script(&keyword, "Keyword");

    let refr = runner.load(refr);

    runner.call_assert(&refr, "HasKeyword", [Value::Object(keyword.clone())], false);
    runner.call(&refr, "AddKeyword", [Value::Object(keyword.clone())]);
    runner.call_assert(&refr, "HasKeyword", [Value::Object(keyword)], true);
}

#[test]
fn object_reference_add_keyword_null() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);

    runner.call_assert(&refr, "HasKeyword", [Value::Null], false);
    runner.call(&refr, "AddKeyword", [Value::Null]);
    runner.call_assert(&refr, "HasKeyword", [Value::Null], false);
}

#[test]
fn object_reference_remove_keyword() {
    let mut builder = TestRunner::builder();

    let keyword = builder.add_record(Keyword::default());

    let item = builder.add_record(MiscObject::default_with(|item| {
        item.keywords = Some(Array(vec![keyword.form]));
    }));
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let keyword = runner.load(keyword);
    let keyword = runner.script(&keyword, "Keyword");

    let refr = runner.load(refr);

    runner.call_assert(&refr, "HasKeyword", [Value::Object(keyword.clone())], true);
    runner.call(&refr, "RemoveKeyword", [Value::Object(keyword.clone())]);
    runner.call_assert(&refr, "HasKeyword", [Value::Object(keyword)], false);
}

#[test]
fn object_reference_remove_keyword_null() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);

    runner.call(&refr, "RemoveKeyword", [Value::Null]);
}

#[test]
fn object_reference_reset_keyword() {
    let mut builder = TestRunner::builder();

    let keyword1 = builder.add_record(Keyword::default());
    let keyword2 = builder.add_record(Keyword::default());

    let item = builder.add_record(MiscObject::default_with(|item| {
        item.keywords = Some(Array(vec![keyword1.form]));
    }));
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let keyword1 = runner.load(keyword1);
    let keyword1 = runner.script(&keyword1, "Keyword");

    let keyword2 = runner.load(keyword2);
    let keyword2 = runner.script(&keyword2, "Keyword");

    let refr = runner.load(refr);

    runner.call(&refr, "RemoveKeyword", [Value::Object(keyword1.clone())]);
    runner.call(&refr, "AddKeyword", [Value::Object(keyword2.clone())]);

    runner.call(&refr, "ResetKeyword", [Value::Object(keyword1.clone())]);
    runner.call(&refr, "ResetKeyword", [Value::Object(keyword2.clone())]);

    runner.call_assert(&refr, "HasKeyword", [Value::Object(keyword1)], true);
    runner.call_assert(&refr, "HasKeyword", [Value::Object(keyword2)], false);
}

#[test]
fn object_reference_reset_keyword_null() {
    let mut builder = TestRunner::builder();

    let item = builder.add_record(MiscObject::default());
    let refr = builder.add_reference_record(PlacedObject::default_with(|obj| {
        obj.base_object = item.form;
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);

    runner.call(&refr, "ResetKeyword", [Value::Null]);
}
