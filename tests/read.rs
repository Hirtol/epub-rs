use epub::doc::EpubDoc;

#[test]
fn read_doc() {
    let input_file = "tests/docs/Metamorphosis-jackson.epub";
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();

    if let Some(title) = doc.mdata("title") {
        println!("Book title: {}", title);
    } else {
        println!("Book title not found");
    }
    println!("Num Pages: {}\n", doc.get_num_pages());

    {
        println!("resources:\n");
        for (k, v) in doc.context.resources.iter() {
            println!("{}: {}\n * {}\n", k, v.mime, v.path.display());
        }
    }

    while let Ok(_) = doc.go_next() {
        println!("ID: {}", doc.get_current_id().unwrap());
        let current = doc.get_current_str();
        match current {
            Ok(v) => println!("Value {:?}\n", v),
            Err(e) => println!("Text Err {:?}\n", e),
        }
    }
}

#[test]
fn read_different_format_epubs() {
    // Read all the epubs in the /epubfiles directory.
    // These are formatted with UTF16/UTF8, borrowed from: https://github.com/tkanai/epub-testfiles
    let files = "tests/docs/epubfiles";

    let paths = std::fs::read_dir(files).unwrap();

    for path in paths {
        println!("Evaluating: {:#?}", path);
        let doc = EpubDoc::new(path.unwrap().path()).unwrap();

        assert!(doc
            .mdata("description")
            .unwrap()
            .contains("Multiple encoding tests"));
    }
}

#[test]
fn bad_epub() {
    //book2.epub has a opf encoded in UTF-16
    //It also has malformed toc, manifest and guide entries, as well as multiple metadata entries
    let input_file = "tests/docs/book2.epub";
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let doc = doc.unwrap();
    if let Some(titles) = doc.context.metadata.get("title") {
        assert_eq!(
            titles.iter().map(|i| i.content.clone()).collect::<Vec<_>>(),
            vec!["Metamorphosis ".to_string(), "Metamorphosis2 ".to_string()]
        );
        println!("Book title: {:#?}", titles);
    } else {
        println!("Book title not found");
    }
}
