use epub::doc::EpubDoc;

#[test]
fn correct_table_of_contents() {
    let input_file = "tests/docs/winnie_the_pooh.epub";
    let doc = EpubDoc::new(input_file).unwrap();

    assert!(
        !doc.context.toc.is_empty(),
        "Table of contents is empty:\n{:#?}",
        doc.context
    );

    let labels = doc
        .context
        .toc
        .into_iter()
        .map(|i| i.label)
        .collect::<Vec<_>>();

    assert!(labels.contains(&"Titlepage".to_string()),)
}

#[test]
fn correct_cover() {
    let input_file = "tests/docs/winnie_the_pooh.epub";
    let doc = EpubDoc::new(input_file).unwrap();

    assert!(
        doc.get_cover_id().is_some(),
        "Error on cover id: {:#?}",
        doc.get_cover_id()
    );

    let cover = doc.get_cover().unwrap();
    let mime = doc
        .context
        .resources
        .get(doc.get_cover_id().unwrap())
        .unwrap();

    assert!(!cover.is_empty());
    assert_eq!(mime.mime, "image/svg+xml");
}
