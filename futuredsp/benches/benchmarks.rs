use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futuredsp::fir::{FirKernel, NonResamplingFirKernel};
use num_complex::Complex;
use rand::Rng;

trait Generatable {
    fn generate() -> Self;
}

impl Generatable for f32 {
    fn generate() -> Self {
        let mut rng = rand::thread_rng();
        rng.gen::<f32>() * 2.0 - 1.0
    }
}

impl Generatable for Complex<f32> {
    fn generate() -> Self {
        let mut rng = rand::thread_rng();
        Complex {
            re: rng.gen::<f32>() * 2.0 - 1.0,
            im: rng.gen::<f32>() * 2.0 - 1.0,
        }
    }
}

fn bench_fir_dynamic_taps<SampleType: Generatable, TapType: Generatable>(
    b: &mut criterion::Bencher,
    ntaps: usize,
    nsamps: usize,
) where
    SampleType: Clone,
    Vec<TapType>: futuredsp::fir::TapsAccessor<TapType = TapType>,
    NonResamplingFirKernel<SampleType, Vec<TapType>>: FirKernel<SampleType>,
{
    let taps: Vec<_> = (0..ntaps).map(|_| TapType::generate()).collect();
    let input: Vec<_> = (0..nsamps + ntaps)
        .map(|_| SampleType::generate())
        .collect();
    let mut output = vec![SampleType::generate(); nsamps];
    let fir = NonResamplingFirKernel::<SampleType, _>::new(black_box(taps));
    b.iter(|| {
        fir.work(black_box(&input), black_box(&mut output));
    });
}

fn bench_fir_static_taps<SampleType: Generatable, TapType: Generatable, const N: usize>(
    b: &mut criterion::Bencher,
    nsamps: usize,
) where
    SampleType: Clone,
    TapType: std::fmt::Debug,
    [TapType; N]: futuredsp::fir::TapsAccessor<TapType = TapType>,
    NonResamplingFirKernel<SampleType, [TapType; N]>: FirKernel<SampleType>,
{
    let taps: Vec<_> = (0..N).map(|_| TapType::generate()).collect();
    let taps: [TapType; N] = taps.try_into().unwrap();
    let input: Vec<_> = (0..nsamps + N).map(|_| SampleType::generate()).collect();
    let mut output = vec![SampleType::generate(); nsamps];
    let fir = NonResamplingFirKernel::<SampleType, _>::new(black_box(taps));
    b.iter(|| {
        fir.work(black_box(&input), black_box(&mut output));
    });
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("fir");

    let nsamps = 1000usize;
    group.throughput(criterion::Throughput::Elements(nsamps as u64));

    for ntaps in [3, 64] {
        group.bench_function(
            format!("fir-{}tap-dynamic real/real {}", ntaps, nsamps),
            |b| {
                bench_fir_dynamic_taps::<f32, f32>(b, ntaps, nsamps);
            },
        );
        group.bench_function(
            format!("fir-{}tap-dynamic complex/real {}", ntaps, nsamps),
            |b| {
                bench_fir_dynamic_taps::<Complex<f32>, f32>(b, ntaps, nsamps);
            },
        );
    }

    // Check some static taps as well
    group.bench_function(format!("fir-3tap-static complex/real {}", nsamps), |b| {
        bench_fir_static_taps::<Complex<f32>, f32, 3>(b, nsamps);
    });
    group.bench_function(format!("fir-64tap-static complex/real {}", nsamps), |b| {
        bench_fir_static_taps::<Complex<f32>, f32, 64>(b, nsamps);
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);