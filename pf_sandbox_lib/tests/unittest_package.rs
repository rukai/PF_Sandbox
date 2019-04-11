use pf_sandbox_lib::package::PackageMeta;

#[test]
fn package_meta_source() {
    let mut meta = PackageMeta::new();

    meta.source = Some(String::from("foo.bar"));
    assert_eq!(meta.url("path").unwrap().as_ref(), "https://foo.bar/path");

    meta.source = Some(String::from("http://foo.bar"));
    assert_eq!(meta.url("path").unwrap().as_ref(), "https://foo.bar/path");

    meta.source = Some(String::from("https://foo.bar"));
    assert_eq!(meta.url("path").unwrap().as_ref(), "https://foo.bar/path");

    meta.source = Some(String::from("/"));
    assert!(meta.url("path").is_none());

    meta.source = Some(String::from(""));
    assert!(meta.url("path").is_none());
}
