//! Renders a mini sparkline using block characters.
use ratatui::{style::Style, text::{Line, Span}};

const BLOCKS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render `data` as a sparkline of `width` characters.
pub fn render(data: &[f64], width: usize, style: Style) -> Line<'static> {
    if data.is_empty() || width == 0 {
        return Line::from(Span::styled(" ".repeat(width), style));
    }

    // Sample data into `width` buckets
    let buckets = sample(data, width);
    let max = buckets.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = buckets.iter().cloned().fold(f64::INFINITY, f64::min);
    let range = (max - min).max(1e-9);

    let chars: String = buckets.iter().map(|&v| {
        let norm  = ((v - min) / range).clamp(0.0, 1.0);
        let idx   = (norm * (BLOCKS.len() - 1) as f64).round() as usize;
        BLOCKS[idx]
    }).collect();

    Line::from(Span::styled(chars, style))
}

fn sample(data: &[f64], width: usize) -> Vec<f64> {
    if data.len() <= width {
        let mut out = vec![0.0f64; width - data.len()];
        out.extend_from_slice(data);
        return out;
    }
    let step = data.len() as f64 / width as f64;
    (0..width).map(|i| {
        let start = (i as f64 * step) as usize;
        let end   = ((i + 1) as f64 * step) as usize;
        let slice = &data[start..end.min(data.len())];
        slice.iter().sum::<f64>() / slice.len() as f64
    }).collect()
}
