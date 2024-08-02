use std::iter;

use plotters::prelude::BitMapBackend;

//const FONT_FAMILY: &str = "DejaVuSansMono"; // somehow crashes on spaces on my system
const FONT_FAMILY: &str = "LiberationMono";

pub fn create_bitmap_buffer(buf: &mut Vec<u8>, x: u32, y: u32) -> BitMapBackend {
    let len = (x * y * 3).try_into().unwrap();

    buf.drain(..);
    buf.extend(iter::repeat(0).take(len)); //[0; (x * y * 3).try_into().unwrap()]; // RGB: bit depth = 24
    assert!(buf.len() == len);

    BitMapBackend::with_buffer(buf.as_mut_slice(), (x, y))
}

pub mod plot {
    

    use anyhow::{bail, Result};
    use chrono::{Duration, NaiveDateTime};
    
    use plotters::{
        prelude::*,
        style::text_anchor::{HPos, Pos, VPos},
    };

    use itertools::Itertools as _;

    use super::FONT_FAMILY;

    const JOBCOUNT_OVER_TIME_TITLE: &str = "Jobcount_(last_48h)";

    #[allow(non_snake_case)]
    const fn TITLE_FONT_SIZE((w, h): (u32, u32)) -> u32 {
        let avg = w + h / 2;
        avg / 28
    }

    pub struct Point<Tx, Ty> {
        x: Tx,
        y: Ty,
    }

    #[allow(non_snake_case)]
    pub fn Point<Tx, Ty>(x: Tx, y: Ty) -> Point<Tx, Ty> {
        Point { x, y }
    }

    impl<Tx, Ty> From<(Tx, Ty)> for Point<Tx, Ty> {
        fn from(value: (Tx, Ty)) -> Self {
            Point(value.0, value.1)
        }
    }

    impl<'a, Tx, Ty> From<&'a (Tx, Ty)> for Point<&'a Tx, &'a Ty> {
        fn from(value: &'a (Tx, Ty)) -> Self {
            Point(&value.0, &value.1)
        }
    }

    fn minmax_by_key<Tx, Ty>(
        dataset: impl Iterator<Item = (Tx, Ty)>,
    ) -> Result<(Point<Tx, Ty>, Point<Tx, Ty>)>
    where
        Tx: Clone,
        Ty: Clone + PartialOrd,
    {
        Ok(match dataset.minmax_by_key(|(x, y)| y.clone()) {
            itertools::MinMaxResult::NoElements => bail!("empty dataset"),
            itertools::MinMaxResult::OneElement((x, y)) => {
                (Point(x.clone(), y.clone()), Point(x, y))
            }
            itertools::MinMaxResult::MinMax((x1, y1), (x2, y2)) => (Point(x1, y1), Point(x2, y2)),
        })
    }

    pub fn jobcount_over_time<DB>(
        backend: DB,
        dataset: &[(NaiveDateTime, usize)],
    ) -> Result<()>
    where
        DB: DrawingBackend,
        DB::ErrorType: 'static,
    {
        let dataset = dataset.iter().sorted_by_key(|(date, _)| date).collect_vec();

        let (min, max) = minmax_by_key(dataset.iter().map(|(a, b)| (a, *b)))?;
        let (first, last): (Point<_, _>, Point<_, _>) = match dataset.as_slice() {
            &[] => bail!("dataset empty"),
            &[singleton] => ((*singleton).into(), (*singleton).into()),
            &[first, .., last] => ((*first).into(), (*last).into()),
        };
        let coord: RangedDateTime<_> = (first.x..last.x).into(); // TODO change formatting to be less verbose so the x axis gets smaller, or use (num hours back from now)

        let drawing_area = backend.into_drawing_area();
        drawing_area.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&drawing_area)
            .caption(
                JOBCOUNT_OVER_TIME_TITLE,
                (FONT_FAMILY, TITLE_FONT_SIZE(drawing_area.dim_in_pixel())).into_font(),
            )
            .margin(5)
            .x_label_area_size(120)
            .y_label_area_size(30)
            .build_cartesian_2d(coord.step(Duration::hours(1)), min.y..max.y)?;
        chart
            .configure_mesh()
            .x_label_style(
                TextStyle::from((FONT_FAMILY, 12).into_font())
                    .pos(Pos::new(HPos::Right, VPos::Default))
                    .transform(FontTransform::Rotate270), // TODO hack transform <= mod90 with sine/cosine
            )
            .draw()?;

        chart.draw_series(LineSeries::new(
            dataset.clone().into_iter().copied(),
            &BLUE,
        ))?;

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .label_font((FONT_FAMILY, 14).into_font())
            .draw()?;

        drawing_area.present()?;

        Ok(())
    }

    // pub struct LocalDateTime(pub DateTime<Local>);

    // impl Deref for LocalDateTime {
    //     type Target = ;

    //     fn deref(&self) -> &Self::Target {
    //         todo!()
    //     }
    // }

    //pub fn

    /*pub fn simple_plot<DB>(
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

    // macro_rules! simple_plot {
    //     ($x_type:ty, $y_type:ty) => {
    //         ::paste::paste! {

    //         #[allow(non_snake_case)]
    //         pub fn [<simple_plot_ $x_type _ $y_type>]<DB>(
    //             backend: DB,
    //             title: impl AsRef<str>,
    //             dataset: &[($x_type, $y_type)],
    //         ) -> Result<()>
    //         where
    //             DB: DrawingBackend,
    //             DB::ErrorType: 'static,
    //         {
    //             let mut dataset = Vec::from(dataset);
    //             //dataset.sort_by_key(|(x, y)| x);
    //             let dataset = dataset;

    //             let (min, max) = match dataset.iter().minmax_by_key(|(x, y)| y) {
    //                 itertools::MinMaxResult::NoElements => bail!("empty dataset"),
    //                 itertools::MinMaxResult::OneElement(&(x, y)) => (Point(x, y), Point(x, y)),
    //                 itertools::MinMaxResult::MinMax(&(x1, y1), &(x2, y2)) => {
    //                     (Point(x1, y1), Point(x2, y2))
    //                 }
    //             };

    //             let drawing_area = backend.into_drawing_area();

    //             drawing_area.fill(&WHITE)?;
    //             let mut chart = ChartBuilder::on(&drawing_area)
    //                 .caption(title, ("sans-serif", 50).into_font())
    //                 .margin(5)
    //                 .x_label_area_size(30)
    //                 .y_label_area_size(30)
    //                 .build_cartesian_2d(min.x..max.x, min.y..max.y)?;

    //             chart.configure_mesh().draw()?;

    //             chart.draw_series(LineSeries::new(dataset.iter().cloned(), &BLUE))?;

    //             chart
    //                 .configure_series_labels()
    //                 .background_style(&WHITE.mix(0.8))
    //                 .border_style(&BLACK)
    //                 .draw()?;

    //             drawing_area.present()?;

    //             Ok(())
    //         }}
    //     };
    // }

    // // Using the macro to generate the function for (f32, f32) and (i32, i32)
    // simple_plot!(f32, f32);
    // simple_plot!(LocalDateTime, usize);
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
