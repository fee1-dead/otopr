use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup, Criterion,
    Throughput,
};
use otopr::VarInt;
use prost::encoding::encoded_len_varint;
use rand::{prelude::StdRng, seq::SliceRandom, SeedableRng};

macro_rules! bench_both {
    ($($b:ident = $t: ident),*) => {$(
        fn $b(group: &mut BenchmarkGroup<WallTime>, values: &[$t], size: &str, otopr: bool) {
            let encoded_len = values
                .iter()
                .cloned()
                .map(|n| n as u64)
                .map(encoded_len_varint)
                .sum::<usize>() as u64;

            if otopr {
                group
                .bench_with_input(size, values, |b, v| {
                    b.iter_batched_ref(move || { Vec::with_capacity(v.len() * 8) }, |buf| {
                        for &value in v {
                            VarInt::write(value, buf);
                        }
                    }, BatchSize::SmallInput)
                })
                .throughput(Throughput::Bytes(encoded_len));
            } else {
                group
                .bench_with_input(size, values, |b, v| {
                    b.iter_batched_ref(move || { Vec::with_capacity(v.len() * 8) }, |buf| {
                        for &value in v {
                            prost::encoding::encode_varint(value as u64, buf);
                        }
                    }, BatchSize::SmallInput)
                })
                .throughput(Throughput::Bytes(encoded_len));
            }
        }
    )*};
}

bench_both!(benchmark_varint_u32 = u32, benchmark_varint_u64 = u64);

pub fn criterion_benchmark(c: &mut Criterion) {
    macro_rules! bench_both {
        ($($group_name:literal, $b: ident, $med_bits:expr, $large_bits: expr, $mixed_upper_bound: expr),*$(,)?) => {$(
            let mut small: Vec<_> = (0..100).collect();
            let mut medium: Vec<_> = (1 << $med_bits..).take(100).collect();
            let mut large: Vec<_> = (1 << $large_bits..).take(100).collect();
            let mut mixed: Vec<_> =  (0..=$mixed_upper_bound)
            .flat_map(move |width| {
                let exponent = width * 7;
                (0..10).map(move |offset| offset + (1 << exponent))
            })
            .collect();

            for v in [&mut small, &mut medium, &mut large, &mut mixed] {
                v.shuffle(&mut StdRng::seed_from_u64(0));
            }

            for group in ["otopr", "prost"] {
                let otopr = group == "otopr";
                let mut group = c.benchmark_group(format!("{}/{}", group, $group_name));

                // Benchmark encoding 100 small (1 byte) varints.
                $b(&mut group, &small, "small", otopr);

                // Benchmark encoding 100 medium (5 byte) varints.
                $b(&mut group, &medium, "medium", otopr);

                // Benchmark encoding 100 large (10 byte) varints.
                $b(&mut group, &large, "large", otopr);

                // Benchmark encoding 100 varints of mixed width (average 5.5 bytes).
                $b(&mut group, &mixed, "mixed", otopr);
            }
        )*}
    }

    bench_both! {
        "varint/encode_u32", benchmark_varint_u32, 15, 28, 4,
        "varint/encode_u64", benchmark_varint_u64, 28, 63, 9,
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
