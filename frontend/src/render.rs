use std::iter;

use plotters::prelude::BitMapBackend;

pub fn create_bitmap_buffer(buf: &mut Vec<u8>, x: u32, y: u32) -> BitMapBackend {
    let len = (x * y * 3).try_into().unwrap();

    buf.drain(..);
    buf.extend(iter::repeat(0).take(len)); //[0; (x * y * 3).try_into().unwrap()]; // RGB: bit depth = 24
    assert!(buf.len() == len);

    BitMapBackend::with_buffer(buf.as_mut_slice(), (x, y))
}

pub mod plot {
    use anyhow::{bail, Result};
    use chrono::NaiveDateTime;
    use ordered_float::OrderedFloat;
    use plotters::prelude::*;

    use itertools::Itertools as _;

    struct Point<Tx, Ty> {
        x: Tx,
        y: Ty,
    }

    #[allow(non_snake_case)]
    fn Point<Tx, Ty>(x: Tx, y: Ty) -> Point<Tx, Ty> {
        Point { x, y }
    }

    /*pub fn simple_plot<DB, CT>(
        backend: DB,
        title: impl AsRef<str>,
        dataset: &[(f32, f32)],
    ) -> Result<()>
    where
        DB: DrawingBackend,
        DB::ErrorType: 'static,
    {
        let (min, max) = match dataset.iter().minmax_by_key(|(x, y)| y) {
            itertools::MinMaxResult::NoElements => bail!("empty dataset"),
            itertools::MinMaxResult::OneElement(&(x, y)) => (Point(x, y), Point(x, y)),
            itertools::MinMaxResult::MinMax(&(x1, y1), &(x2, y2)) => (Point(x1, y1), Point(x2, y2)),
        };

        let drawing_area = backend.into_drawing_area();

        drawing_area.fill(&WHITE)?;
        let mut chart = ChartBuilder::on(&drawing_area)
            .caption(title, ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..dataset.len() as f32, min.y..max.y)?;

        chart.configure_mesh().draw()?;

        /*chart
        .draw_series(LineSeries::new(
            (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
            &RED,
        ))?
        .label("y = x^2")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));*/

        chart.draw_series(LineSeries::new(dataset, &BLUE))?;

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        drawing_area.present()?;

        Ok(())
    }*/

    macro_rules! simple_plot {
        ($x_type:ty, $y_type:ty) => {
            ::paste::paste! {

            #[allow(non_snake_case)]
            pub fn [<simple_plot_ $x_type _ $y_type>]<DB>(
                backend: DB,
                title: impl AsRef<str>,
                dataset: &[($x_type, $y_type)],
            ) -> Result<()>
            where
                DB: DrawingBackend,
                DB::ErrorType: 'static,
            {
                let mut dataset = Vec::from(dataset);
                //dataset.sort_by_key(|(x, y)| x);
                let dataset = dataset;

                let (min, max) = match dataset.iter().minmax_by_key(|(x, y)| y) {
                    itertools::MinMaxResult::NoElements => bail!("empty dataset"),
                    itertools::MinMaxResult::OneElement(&(x, y)) => (Point(x, y), Point(x, y)),
                    itertools::MinMaxResult::MinMax(&(x1, y1), &(x2, y2)) => {
                        (Point(x1, y1), Point(x2, y2))
                    }
                };

                let drawing_area = backend.into_drawing_area();

                drawing_area.fill(&WHITE)?;
                let mut chart = ChartBuilder::on(&drawing_area)
                    .caption(title, ("sans-serif", 50).into_font())
                    .margin(5)
                    .x_label_area_size(30)
                    .y_label_area_size(30)
                    .build_cartesian_2d(min.x..max.x, min.y..max.y)?;

                chart.configure_mesh().draw()?;

                chart.draw_series(LineSeries::new(dataset.iter().cloned(), &BLUE))?;

                chart
                    .configure_series_labels()
                    .background_style(&WHITE.mix(0.8))
                    .border_style(&BLACK)
                    .draw()?;

                drawing_area.present()?;

                Ok(())
            }}
        };
    }

    // Using the macro to generate the function for (f32, f32) and (i32, i32)
    simple_plot!(f32, f32);
    //simple_plot!(NaiveDateTime, usize);
}

// fn test_generic_plot<DB, X, Y>(backend: DB, dataset: &[(X, Y)]) -> Result<()>
// where
//     X: Ranged + PartialEq + PartialOrd + Clone + Debug + Display,
//     Y: Ranged + PartialEq + PartialOrd + Clone + Debug + Display,
//     RangeInclusive<X>: Ranged + ValueFormatter<X>,
//     RangeInclusive<Y>: Ranged + ValueFormatter<Y>,
//     <RangeInclusive<X> as Ranged>::ValueType: std::fmt::Debug,
//     <RangeInclusive<Y> as Ranged>::ValueType: std::fmt::Debug,
//     DB: DrawingBackend,
// {
//     use plotters::prelude::*;
//     let (ymin, ymax) = match dataset.iter().minmax_by_key(|(x, y)| y) {
//         itertools::MinMaxResult::NoElements => bail!("empty dataset"),
//         itertools::MinMaxResult::OneElement(&(x, y)) => ((x, y), (x, y)),
//         itertools::MinMaxResult::MinMax(&(x1, y1), &(x2, y2)) => ((x1, y1), (x2, y2)),
//     };
//     let (xmin, xmax) = match (dataset.first(), dataset.last()) {
//         (Some(first), Some(last)) => (first, last),
//         _ => bail!("empty dataset"),
//     };

//     let drawing_area = backend.into_drawing_area();

//     drawing_area.fill(&WHITE)?;
//     let mut chart = ChartBuilder::on(&drawing_area)
//         .caption("test", ("sans-serif", 50).into_font())
//         .margin(5)
//         .x_label_area_size(30)
//         .y_label_area_size(30)
//         .build_cartesian_2d(xmin.0..=xmax.0, ymin.1..=ymax.1)?;

//     chart.configure_mesh().draw()?;

//     todo!()
// }
