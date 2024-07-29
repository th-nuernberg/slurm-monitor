use std::{
    fmt::{Debug, Display},
    ops::RangeInclusive,
};

use anyhow::{bail, Result};
use itertools::Itertools as _;
use plotters::{backend::DrawingBackend, coord::ranged1d::ValueFormatter, prelude::Ranged};

pub mod plot {
    use anyhow::{bail, Result};
    use plotters::{element::PointCollection, prelude::*};
    use std::path::Path;

    use itertools::Itertools as _;

    struct Point<T> {
        x: T,
        y: T,
    }

    #[allow(non_snake_case)]
    fn Point<T>(x: T, y: T) -> Point<T> {
        Point { x, y }
    }

    pub fn simple_plot<DB, CT>(
        backend: DB,
        title: impl AsRef<str>,
        chart: ChartContext<'_, DB, CT>,
    ) -> Result<()>
    where
        DB: DrawingBackend,
        DB::ErrorType: 'static,
        CT: CoordTranslate,
    {
        let dataset = dataset.collect_vec();
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
    }
}

fn test_generic_plot<DB, X, Y>(backend: DB, dataset: &[(X, Y)]) -> Result<()>
where
    X: Ranged + PartialEq + PartialOrd + Clone + Debug + Display,
    Y: Ranged + PartialEq + PartialOrd + Clone + Debug + Display,
    RangeInclusive<X>: Ranged + ValueFormatter<X>,
    RangeInclusive<Y>: Ranged + ValueFormatter<Y>,
    <RangeInclusive<X> as Ranged>::ValueType: std::fmt::Debug,
    <RangeInclusive<Y> as Ranged>::ValueType: std::fmt::Debug,
    DB: DrawingBackend,
{
    use plotters::prelude::*;
    let (ymin, ymax) = match dataset.iter().minmax_by_key(|(x, y)| y) {
        itertools::MinMaxResult::NoElements => bail!("empty dataset"),
        itertools::MinMaxResult::OneElement(&(x, y)) => ((x, y), (x, y)),
        itertools::MinMaxResult::MinMax(&(x1, y1), &(x2, y2)) => ((x1, y1), (x2, y2)),
    };
    let (xmin, xmax) = match (dataset.first(), dataset.last()) {
        (Some(first), Some(last)) => (first, last),
        _ => bail!("empty dataset"),
    };

    let drawing_area = backend.into_drawing_area();

    drawing_area.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&drawing_area)
        .caption("test", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(xmin.0..=xmax.0, ymin.1..=ymax.1)?;

    chart.configure_mesh().draw()?;

    todo!()
}
