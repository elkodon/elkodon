use elkodon_bb_container::semantic_string::SemanticString;
use elkodon_bb_posix::config::*;
use elkodon_bb_posix::file::*;
use elkodon_bb_posix::file_descriptor::*;
use elkodon_bb_posix::unique_system_id::UniqueSystemId;
use elkodon_bb_system_types::file_name::FileName;
use elkodon_bb_system_types::file_path::FilePath;
use elkodon_bb_testing::assert_that;
use elkodon_bb_testing::test_requires;
use elkodon_pal_posix::posix::POSIX_SUPPORT_PERMISSIONS;
use elkodon_pal_posix::posix::POSIX_SUPPORT_USERS_AND_GROUPS;

fn generate_file_name() -> FilePath {
    let mut file = FileName::new(b"file_tests").unwrap();
    file.push_bytes(
        UniqueSystemId::new()
            .unwrap()
            .value()
            .to_string()
            .as_bytes(),
    )
    .unwrap();

    FilePath::from_path_and_file(&TEMP_DIRECTORY, &file).unwrap()
}

struct TestFixture {
    file: FilePath,
}

impl TestFixture {
    fn new() -> TestFixture {
        let file = generate_file_name();
        File::remove(&file).ok();
        TestFixture { file }
    }

    fn file(&self) -> &FilePath {
        &self.file
    }

    fn create_file(&self, name: &FilePath) -> File {
        let file = FileBuilder::new(name)
            .creation_mode(CreationMode::PurgeAndCreate)
            .create();

        assert_that!(file, is_ok);
        file.unwrap()
    }

    fn open_file(&self, name: &FilePath) -> File {
        let file = FileBuilder::new(name).open_existing(AccessMode::ReadWrite);

        assert_that!(file, is_ok);
        file.unwrap()
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        File::remove(self.file()).expect("failed to cleanup test file");
    }
}

#[test]
fn file_opening_non_existing_file_fails() {
    let test = TestFixture::new();
    let result = FileBuilder::new(test.file()).open_existing(AccessMode::ReadWrite);

    assert_that!(result, is_err);
    assert_that!(result.err().unwrap(), eq FileOpenError::FileDoesNotExist);
}

#[test]
fn file_creating_non_existing_file_succeeds() {
    let test = TestFixture::new();
    let result = FileBuilder::new(test.file())
        .creation_mode(CreationMode::CreateExclusive)
        .create();

    assert_that!(result, is_ok);
}

#[test]
fn file_creating_existing_file_fails() {
    let test = TestFixture::new();

    test.create_file(test.file());

    let result = FileBuilder::new(test.file())
        .creation_mode(CreationMode::CreateExclusive)
        .create();

    assert_that!(result, is_err);
    assert_that!(result.err().unwrap(), eq FileCreationError::FileAlreadyExists);
}

#[test]
fn file_purge_and_create_non_existing_file_succeeds() {
    let test = TestFixture::new();

    let result = FileBuilder::new(test.file())
        .creation_mode(CreationMode::PurgeAndCreate)
        .create();

    assert_that!(result, is_ok);
}

#[test]
fn file_purge_and_create_existing_file_succeeds() {
    let test = TestFixture::new();
    test.create_file(test.file());

    let result = FileBuilder::new(test.file())
        .creation_mode(CreationMode::PurgeAndCreate)
        .create();

    assert_that!(result, is_ok);
}

#[test]
fn file_open_or_create_with_existing_file_succeeds() {
    let test = TestFixture::new();

    test.create_file(test.file());

    let result = FileBuilder::new(&test.file)
        .creation_mode(CreationMode::OpenOrCreate)
        .create();

    assert_that!(result, is_ok);
}

#[test]
fn file_open_or_create_with_non_existing_file_succeeds() {
    let test = TestFixture::new();

    let result = FileBuilder::new(&test.file)
        .creation_mode(CreationMode::OpenOrCreate)
        .create();

    assert_that!(result, is_ok);
}

#[test]
fn file_creating_file_applies_additional_settings() {
    test_requires!(POSIX_SUPPORT_PERMISSIONS && POSIX_SUPPORT_USERS_AND_GROUPS);

    let test = TestFixture::new();

    let file = FileBuilder::new(&test.file)
        .creation_mode(CreationMode::OpenOrCreate)
        .permission(Permission::OWNER_READ)
        .create();

    assert_that!(file, is_ok);

    let file = file.ok().unwrap();
    assert_that!(
        file.metadata().unwrap().permission(), eq
        Permission::OWNER_READ
    );
}

#[test]
fn file_simple_read_write_works() {
    let test = TestFixture::new();
    let mut file = test.create_file(&test.file);

    let mut content = "oh look what is in the file \n in in that line \t fuuu".to_string();
    let result = file.write(unsafe { content.as_mut_vec() }.as_slice());

    assert_that!(result, is_ok);
    assert_that!(content, len result.ok().unwrap() as usize);

    let mut read_content = String::new();
    let result = file.read_to_string(&mut read_content);
    assert_that!(result, is_ok);
    assert_that!(content, len result.ok().unwrap() as usize);

    assert_that!(content, eq read_content);
}

#[test]
fn file_two_file_objects_read_work_with_ranges_in_same_file() {
    let test = TestFixture::new();
    let mut file_a = test.create_file(&test.file);
    let mut file_b = test.open_file(&test.file);

    let mut content = "hello".to_string();
    let result = file_a.write(unsafe { content.as_mut_vec() }.as_slice());
    assert_that!(result, is_ok);
    assert_that!(content, len result.ok().unwrap() as usize);

    let mut content = "world".to_string();
    let result = file_b.write_at(2, unsafe { content.as_mut_vec() }.as_slice());
    assert_that!(result, is_ok);
    assert_that!(content, len result.ok().unwrap() as usize);

    let mut read_content = String::new();
    let result = file_a.read_range_to_string(1, 7, &mut read_content);
    assert_that!(result, is_ok);
    assert_that!(result.ok().unwrap(), eq 6);

    assert_that!("eworld", eq read_content);
}

#[test]
fn file_created_file_does_exist() -> Result<(), FileError> {
    let test = TestFixture::new();
    test.create_file(&test.file);

    assert_that!(File::does_exist(&test.file)?, eq true);
    Ok(())
}

#[test]
fn file_non_existing_file_does_not_exist() -> Result<(), FileError> {
    let test = TestFixture::new();

    assert_that!(!File::does_exist(&test.file)?, eq true);
    Ok(())
}

#[test]
fn file_remove_returns_true_when_file_exists() -> Result<(), FileError> {
    let test = TestFixture::new();
    test.create_file(&test.file);

    assert_that!(File::remove(&test.file)?, eq true);
    Ok(())
}

#[test]
fn file_remove_returns_false_when_file_not_exists() -> Result<(), FileError> {
    let test = TestFixture::new();

    assert_that!(!File::remove(&test.file)?, eq true);
    Ok(())
}
