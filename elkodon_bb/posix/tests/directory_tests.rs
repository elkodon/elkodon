use elkodon_bb_container::byte_string::FixedSizeByteString;
use elkodon_bb_container::semantic_string::SemanticString;
use elkodon_bb_posix::config::*;
use elkodon_bb_posix::directory::*;
use elkodon_bb_posix::file::*;
use elkodon_bb_posix::file_type::*;
use elkodon_bb_posix::unique_system_id::UniqueSystemId;
use elkodon_bb_system_types::file_name::FileName;
use elkodon_bb_system_types::file_path::FilePath;
use elkodon_bb_system_types::path::Path;
use elkodon_bb_testing::assert_that;
use elkodon_pal_settings::PATH_SEPARATOR;

struct TestFixture {
    files: Vec<FilePath>,
    directories: Vec<Path>,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        for file in &self.files {
            File::remove(file).expect("failed to cleanup test file");
        }

        for dir in self.directories.iter().rev() {
            Directory::remove(dir).expect("failed to cleanup test directory");
        }
    }
}

impl TestFixture {
    fn new() -> Self {
        Self {
            files: vec![],
            directories: vec![],
        }
    }
    fn create_file(&mut self, directory: &Path) -> File {
        let mut file = FileName::new(b"dir_tests_file").unwrap();
        file.push_bytes(
            UniqueSystemId::new()
                .unwrap()
                .value()
                .to_string()
                .as_bytes(),
        )
        .unwrap();

        let file = FilePath::from_path_and_file(directory, &file).unwrap();

        self.files.push(file);

        FileBuilder::new(&file)
            .creation_mode(CreationMode::PurgeAndCreate)
            .create()
            .unwrap()
    }

    fn create_dir(&mut self, directory: &Path) -> Directory {
        let mut directory = *directory;
        let mut file = FixedSizeByteString::from_bytes(b"dir_tests_").unwrap();
        file.push_bytes(
            UniqueSystemId::new()
                .unwrap()
                .value()
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        directory.add_path_entry(&file).unwrap();

        self.directories.push(directory);

        Directory::create(&directory, Permission::OWNER_ALL).unwrap()
    }

    fn generate_directory_name(&mut self) -> Path {
        let mut directory = TEMP_DIRECTORY;
        directory.push(PATH_SEPARATOR).unwrap();
        directory.push_bytes(b"dir_tests_").unwrap();
        directory
            .push_bytes(
                UniqueSystemId::new()
                    .unwrap()
                    .value()
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
        self.directories.push(directory);

        directory
    }
}

#[test]
fn directory_temp_directory_does_exist() {
    assert_that!(Directory::does_exist(&TEMP_DIRECTORY).unwrap(), eq true);
}

#[test]
fn directory_non_existing_directory_does_not_exist() {
    assert_that!(!Directory::does_exist(&Path::new(b"i_do_not_exist").unwrap()).unwrap(), eq true);
}

#[test]
fn directory_file_is_not_a_directory() {
    FileBuilder::new(&FilePath::new(b"no_directory").unwrap())
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();
    assert_that!(Directory::does_exist(&Path::new(b"no_directory").unwrap()).unwrap(), eq false);
    File::remove(&FilePath::new(b"no_directory").unwrap()).unwrap();
}

#[test]
fn directory_create_from_path_works() {
    let mut test = TestFixture::new();

    let sut_name = test.generate_directory_name();

    assert_that!(Directory::does_exist(&sut_name).unwrap(), eq false);
    let sut_create = Directory::create(&sut_name, Permission::OWNER_ALL);
    assert_that!(sut_create, is_ok);
    assert_that!(Directory::does_exist(&sut_name).unwrap(), eq true);
}

#[test]
fn directory_create_from_path_works_recursively() {
    let mut test = TestFixture::new();

    let mut sut_name = test.generate_directory_name();
    sut_name
        .add_path_entry(&FixedSizeByteString::from_bytes(b"all").unwrap())
        .unwrap();
    sut_name
        .add_path_entry(&FixedSizeByteString::from_bytes(b"glory").unwrap())
        .unwrap();
    sut_name
        .add_path_entry(&FixedSizeByteString::from_bytes(b"to").unwrap())
        .unwrap();
    sut_name
        .add_path_entry(&FixedSizeByteString::from_bytes(b"the").unwrap())
        .unwrap();
    sut_name
        .add_path_entry(&FixedSizeByteString::from_bytes(b"hypnotoad").unwrap())
        .unwrap();

    assert_that!(Directory::does_exist(&sut_name).unwrap(), eq false);
    let sut_create = Directory::create(&sut_name, Permission::OWNER_ALL);
    assert_that!(sut_create, is_ok);
    assert_that!(Directory::does_exist(&sut_name).unwrap(), eq true);
}

#[test]
fn directory_open_from_path_works() {
    let mut test = TestFixture::new();

    let sut_name = test.generate_directory_name();

    Directory::create(&sut_name, Permission::OWNER_ALL).unwrap();

    let sut_open = Directory::new(&sut_name);
    assert_that!(sut_open, is_ok);
}

#[test]
fn directory_list_contents_works() {
    let mut test = TestFixture::new();

    let sut_name = test.generate_directory_name();

    let sut = Directory::create(&sut_name, Permission::OWNER_ALL);
    assert_that!(sut, is_ok);
    let sut = sut.unwrap();

    let mut dir_vec = vec![];
    const NUMBER_OF_DIRECTORIES: usize = 10;
    for _i in 0..NUMBER_OF_DIRECTORIES {
        let dir = test.create_dir(sut.path());
        dir_vec.push(dir.path().to_string());
    }

    let mut file_vec = vec![];
    const NUMBER_OF_FILES: usize = 10;
    for _i in 0..NUMBER_OF_FILES {
        let file = test.create_file(sut.path());
        file_vec.push(file.path().unwrap().to_string());
    }

    let content = sut.contents().unwrap();
    assert_that!(content, len NUMBER_OF_DIRECTORIES + NUMBER_OF_FILES);

    let is_part_of_dir = |name: String| -> bool {
        for dir in &dir_vec {
            let separator = String::from_utf8_lossy(&[PATH_SEPARATOR; 1]);
            if *dir == sut.path().to_string() + &separator + &name {
                return true;
            }
        }
        false
    };

    let is_part_of_files = |name: String| -> bool {
        for file in &file_vec {
            let separator = String::from_utf8_lossy(&[PATH_SEPARATOR; 1]);
            if *file == sut.path().to_string() + &separator + &name {
                return true;
            }
        }
        false
    };

    for i in 0..NUMBER_OF_DIRECTORIES {
        assert_that!(is_part_of_dir(content[i].name().to_string()), eq true);
        assert_that!(content[i].metadata().file_type(), eq FileType::Directory);
    }

    for i in 0..NUMBER_OF_FILES {
        assert_that!(is_part_of_files(
            content[i + NUMBER_OF_DIRECTORIES].name().to_string()
        ), eq true);
        assert_that!(
            content[i + NUMBER_OF_DIRECTORIES].metadata().file_type(),
            eq FileType::File
        );
    }
}
