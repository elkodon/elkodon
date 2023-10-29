use elkodon_bb_container::semantic_string::*;
use elkodon_bb_system_types::file_name::FileName;
use elkodon_bb_system_types::path::*;
use elkodon_bb_testing::assert_that;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    #[test]
    fn path_new_with_illegal_name_fails() {
        let sut = Path::new(b"\0a");
        assert_that!(sut, is_err);

        let sut = Path::new(b";?!@");
        assert_that!(sut, is_err);

        let sut = Path::new(b"\\weird\\&^relative!@#$%^&*()\\path\\..");
        assert_that!(sut, is_err);
    }

    #[test]
    fn path_new_with_legal_name_works() {
        let sut = Path::new(b"C:\\some\\file\\path");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"C:\\some\\file\\p\\");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"C:\\some\\file\\.p\\");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"C:\\some\\file\\p\\.\\");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"C:\\some\\file\\p\\..\\");
        assert_that!(sut, is_ok);
    }

    #[test]
    fn path_add_works() {
        let mut sut = Path::new(b"C:\\some").unwrap();
        sut.add_path_entry(&FileName::new(b"file").unwrap())
            .unwrap();
        sut.add_path_entry(&FileName::new(b"path").unwrap())
            .unwrap();
        assert_that!(sut, eq b"C:\\some\\file\\path");

        let mut sut = Path::new(b"").unwrap();
        sut.add_path_entry(&FileName::new(b"another").unwrap())
            .unwrap();
        sut.add_path_entry(&FileName::new(b"testy").unwrap())
            .unwrap();
        assert_that!(sut, eq b"another\\testy");

        let mut sut = Path::new(b"fuu\\").unwrap();
        sut.add_path_entry(&FileName::new(b"blaaaha").unwrap())
            .unwrap();
        sut.add_path_entry(&FileName::new(b"blub.ma").unwrap())
            .unwrap();
        assert_that!(sut, eq b"fuu\\blaaaha\\blub.ma");
    }
}

#[cfg(not(target_os = "windows"))]
mod unix {
    use super::*;

    #[test]
    fn path_new_with_illegal_name_fails() {
        let sut = Path::new(b"\0a");
        assert_that!(sut, is_err);

        let sut = Path::new(b";?!@");
        assert_that!(sut, is_err);

        let sut = Path::new(b"/weird/&^relative!@#$%^&*()/path/..");
        assert_that!(sut, is_err);
    }

    #[test]
    fn path_new_with_legal_name_works() {
        let sut = Path::new(b"/some/file/path");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"/some/file/p/");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"/some/file/.p/");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"/some/file/p/./");
        assert_that!(sut, is_ok);

        let sut = Path::new(b"/some/file/p/../");
        assert_that!(sut, is_ok);
    }

    #[test]
    fn path_add_works() {
        let mut sut = Path::new(b"/some").unwrap();
        sut.add_path_entry(&FileName::new(b"file").unwrap())
            .unwrap();
        sut.add_path_entry(&FileName::new(b"path").unwrap())
            .unwrap();
        assert_that!(sut, eq b"/some/file/path");

        let mut sut = Path::new(b"").unwrap();
        sut.add_path_entry(&FileName::new(b"another").unwrap())
            .unwrap();
        sut.add_path_entry(&FileName::new(b"testy").unwrap())
            .unwrap();
        assert_that!(sut, eq b"another/testy");

        let mut sut = Path::new(b"fuu/").unwrap();
        sut.add_path_entry(&FileName::new(b"blaaaha").unwrap())
            .unwrap();
        sut.add_path_entry(&FileName::new(b"blub.ma").unwrap())
            .unwrap();
        assert_that!(sut, eq b"fuu/blaaaha/blub.ma");
    }

    #[test]
    fn path_list_all_entries_works() {
        let sut = Path::new(b"/some/file/path/").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 3);
        assert_that!(entries[0], eq b"some");
        assert_that!(entries[1], eq b"file");
        assert_that!(entries[2], eq b"path");

        let sut = Path::new(b"no/path/separator/front/").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 4);
        assert_that!(entries[0], eq b"no");
        assert_that!(entries[1], eq b"path");
        assert_that!(entries[2], eq b"separator");
        assert_that!(entries[3], eq b"front");

        let sut = Path::new(b"/no/path/separator/back").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 4);
        assert_that!(entries[0], eq b"no");
        assert_that!(entries[1], eq b"path");
        assert_that!(entries[2], eq b"separator");
        assert_that!(entries[3], eq b"back");

        let sut = Path::new(b"no/path/separator/front_and_back").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 4);
        assert_that!(entries[0], eq b"no");
        assert_that!(entries[1], eq b"path");
        assert_that!(entries[2], eq b"separator");
        assert_that!(entries[3], eq b"front_and_back");

        let sut = Path::new(b"single_entry_1").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 1);
        assert_that!(entries[0], eq b"single_entry_1");

        let sut = Path::new(b"single_entry_2/").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 1);
        assert_that!(entries[0], eq b"single_entry_2");

        let sut = Path::new(b"/single_entry_3").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 1);
        assert_that!(entries[0], eq b"single_entry_3");

        let sut = Path::new(b"/single_entry_4/").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 1);
        assert_that!(entries[0], eq b"single_entry_4");

        let sut = Path::new(b"////slashes_everywhere////").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 1);
        assert_that!(entries[0], eq b"slashes_everywhere");

        let sut = Path::new(b"//slashes///everywhere////oh_no").unwrap();
        let entries = sut.entries();
        assert_that!(entries, len 3);
        assert_that!(entries[0], eq b"slashes");
        assert_that!(entries[1], eq b"everywhere");
        assert_that!(entries[2], eq b"oh_no");
    }
}
