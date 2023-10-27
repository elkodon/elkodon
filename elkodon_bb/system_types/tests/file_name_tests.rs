use elkodon_bb_container::semantic_string::*;
use elkodon_bb_system_types::file_name::*;
use elkodon_bb_testing::assert_that;

#[test]
fn file_name_new_with_illegal_name_fails() {
    let sut = FileName::new(b"");
    assert_that!(sut, is_err);

    let sut = FileName::new(b".");
    assert_that!(sut, is_err);

    let sut = FileName::new(b"..");
    assert_that!(sut, is_err);
}

#[test]
fn file_name_new_name_with_slash_is_illegal() {
    let sut = FileName::new(b"hell/.txt");
    assert_that!(sut, is_err);
}

#[test]
fn file_name_pop_fails_when_it_results_in_illegal_name() {
    let sut = FileName::new(b"..f");
    assert_that!(sut, is_ok);
    let mut sut = sut.unwrap();

    assert_that!(sut.pop(), is_err);

    assert_that!(sut, len 3);
    assert_that!(sut.as_bytes(), eq b"..f");
}

#[test]
fn file_name_remove_fails_when_it_results_in_illegal_name() {
    let sut = FileName::new(b".f.");
    assert_that!(sut, is_ok);
    let mut sut = sut.unwrap();

    assert_that!(sut.remove(1), is_err);

    assert_that!(sut, len 3);
    assert_that!(sut.as_bytes(), eq b".f.");
}

#[test]
fn file_name_remove_range_fails_when_it_results_in_illegal_name() {
    let sut = FileName::new(b".fuu");
    assert_that!(sut, is_ok);
    let mut sut = sut.unwrap();

    assert_that!(sut.remove_range(1, 3), is_err);

    assert_that!(sut, len 4);
    assert_that!(sut.as_bytes(), eq b".fuu");
}

#[test]
fn file_name_retain_fails_when_it_results_in_illegal_name() {
    let sut = FileName::new(b".fuu");
    assert_that!(sut, is_ok);
    let mut sut = sut.unwrap();

    let retain_result = sut.retain(|c| c != b'.');
    assert_that!(retain_result, is_err);

    assert_that!(sut, len 4);
    assert_that!(sut.as_bytes(), eq b".fuu");
}
