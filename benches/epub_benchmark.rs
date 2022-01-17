use criterion::{criterion_group, criterion_main, Criterion};
use epub::doc::EpubDoc;

fn epub_open_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Epub Open");

    group.bench_function("Read Charles Dickens", |bencher| {
        bencher.iter_with_large_drop(|| {
            // Just read and initialize the epub
            let _ = EpubDoc::new("tests/docs/charles-dickens_a-christmas-carol.epub").unwrap();
        })
    });

    group.finish();
}

fn epub_grab_resource_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Epub Grab Resource");

    group.bench_function("Read Charles Dickens", |bencher| {
        bencher.iter_with_large_drop(|| {
            let mut doc =
                EpubDoc::new("tests/docs/charles-dickens_a-christmas-carol.epub").unwrap();
            let _ = doc.get_resource("chapter-1.xhtml").unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, epub_open_benchmark, epub_grab_resource_benchmark);

criterion_main!(benches);
