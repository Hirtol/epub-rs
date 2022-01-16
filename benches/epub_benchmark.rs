use std::fs::read;
use std::path::Path;

use criterion::measurement::WallTime;
use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};
use epub::doc::EpubDoc;


fn epub_open_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Epub Open");

    group.bench_function("Read Charles Dickens", |bencher| {
        bencher.iter_with_large_drop(|| {
            // Just read and initialize the epub
            let doc = EpubDoc::new("tests/docs/charles-dickens_a-christmas-carol.epub").unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, epub_open_benchmark);

criterion_main!(benches);
