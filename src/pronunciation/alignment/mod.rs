use std::time::Duration;

use ndarray::ArrayView1;

use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, AlignmentWeights, PronunciationError, PronunciationFeatures,
    Result,
};

#[derive(Debug)]
pub struct AudioAligner {
    warp_band: usize,
    segment_frames: usize,
    weights: AlignmentWeights,
}

impl AudioAligner {
    pub fn new(weights: AlignmentWeights) -> Self {
        Self {
            warp_band: DEFAULT_WARP_BAND,
            segment_frames: DEFAULT_SEGMENT_FRAMES,
            weights,
        }
    }

    pub fn align(
        &self,
        reference: &PronunciationFeatures,
        learner: &PronunciationFeatures,
    ) -> Result<AlignmentReport> {
        ensure_features(reference, learner)?;
        let score_grid = build_cost_grid(reference, learner, self.warp_band, &self.weights);
        let path = trace_optimal_path(&score_grid)?;
        let segments =
            summarise_segments(reference, learner, &path, self.segment_frames, &score_grid)?;
        Ok(AlignmentReport {
            phonemes: segments.phonemes,
            total_duration: Duration::from_millis(frame_to_ms(reference.frame_count).round() as u64),
            reference_path_cost: segments.total_cost,
            learner_path_cost: segments.total_cost,
            global_time_offset_ms: segments.global_offset,
            confidence: segments.confidence,
            reference_energy: reference.energy.to_vec(),
            learner_energy: learner.energy.to_vec(),
            similarity_band: segments.similarity_band,
        })
    }
}

const FRAME_HOP_MS: f32 = 10.0;
const DEFAULT_WARP_BAND: usize = 20;
const DEFAULT_SEGMENT_FRAMES: usize = 18;
const COST_NORMALISER: f32 = 6.0;

fn ensure_features(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
) -> Result<()> {
    if reference.frame_count == 0 {
        return Err(PronunciationError::new(
            "reference features contain no frames for alignment",
        ));
    }
    if learner.frame_count == 0 {
        return Err(PronunciationError::new(
            "learner features contain no frames for alignment",
        ));
    }
    Ok(())
}

fn frame_to_ms(frames: usize) -> f32 {
    frames as f32 * FRAME_HOP_MS
}

fn build_cost_grid(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
    band: usize,
    weights: &AlignmentWeights,
) -> Vec<Vec<Cell>> {
    let mut grid = vec![vec![Cell::invalid(); learner.frame_count]; reference.frame_count];
    for row in 0..reference.frame_count {
        fill_row(reference, learner, band, row, weights, &mut grid);
    }
    grid
}

fn fill_row(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
    band: usize,
    row: usize,
    weights: &AlignmentWeights,
    grid: &mut [Vec<Cell>],
) {
    let start = row.saturating_sub(band);
    let end = (row + band + 1).min(learner.frame_count);
    for col in start..end {
        let local = frame_cost(reference, learner, row, col, weights);
        grid[row][col] = update_cell(local, row, col, grid);
    }
}

fn update_cell(local: f32, row: usize, col: usize, grid: &[Vec<Cell>]) -> Cell {
    if row == 0 && col == 0 {
        return Cell::origin(local);
    }
    let mut best = Step::new(f32::INFINITY, Direction::Origin);
    if row > 0 && col > 0 {
        best = Step::better(best, grid[row - 1][col - 1], Direction::Diagonal);
    }
    if row > 0 {
        best = Step::better(best, grid[row - 1][col], Direction::Up);
    }
    if col > 0 {
        best = Step::better(best, grid[row][col - 1], Direction::Left);
    }
    if !best.cost.is_finite() {
        return Cell::invalid();
    }
    Cell::with_prev(local + best.cost, local, best.direction)
}

fn frame_cost(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
    row: usize,
    col: usize,
    weights: &AlignmentWeights,
) -> f32 {
    let mfcc = mean_abs(reference.mfcc.row(row), learner.mfcc.row(col));
    let delta = mean_abs(reference.deltas.row(row), learner.deltas.row(col));
    let delta_delta = mean_abs(
        reference.delta_deltas.row(row),
        learner.delta_deltas.row(col),
    );
    let mel = mean_abs(
        reference.mel_spectrogram.row(row),
        learner.mel_spectrogram.row(col),
    );
    let energy = (reference.energy[row] - learner.energy[col]).abs();
    let flux = (reference.spectral_flux[row] - learner.spectral_flux[col]).abs();
    (mfcc * weights.mfcc
        + delta * weights.delta
        + delta_delta * weights.delta_delta
        + mel * weights.mel
        + energy * weights.energy
        + flux * weights.flux)
        .min(COST_NORMALISER)
}

fn mean_abs(lhs: ArrayView1<'_, f32>, rhs: ArrayView1<'_, f32>) -> f32 {
    if lhs.is_empty() || rhs.is_empty() {
        return 0.0;
    }
    lhs.iter()
        .zip(rhs.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / lhs.len().min(rhs.len()) as f32
}

fn trace_optimal_path(grid: &[Vec<Cell>]) -> Result<Vec<Point>> {
    let (mut row, mut col) = terminal_cell(grid)?;
    let mut path = Vec::new();
    while row > 0 || col > 0 {
        path.push(Point {
            row,
            col,
            cost: grid[row][col].local,
        });
        match grid[row][col].direction {
            Direction::Diagonal => {
                row -= 1;
                col -= 1;
            }
            Direction::Up => row -= 1,
            Direction::Left => col -= 1,
            Direction::Origin => break,
        }
    }
    path.push(Point {
        row: 0,
        col: 0,
        cost: grid[0][0].local,
    });
    path.reverse();
    Ok(path)
}

fn terminal_cell(grid: &[Vec<Cell>]) -> Result<(usize, usize)> {
    let last_row = grid.len().saturating_sub(1);
    let last_col = grid
        .first()
        .map(|row| row.len().saturating_sub(1))
        .unwrap_or(0);
    let mut best = (last_row, last_col, f32::INFINITY);
    for (col, cell) in grid[last_row].iter().enumerate().take(last_col + 1) {
        if cell.cost < best.2 {
            best = (last_row, col, cell.cost);
        }
    }
    for (row, column) in grid.iter().enumerate().take(last_row + 1) {
        let cost = column[last_col].cost;
        if cost < best.2 {
            best = (row, last_col, cost);
        }
    }
    if !best.2.is_finite() {
        return Err(PronunciationError::new(
            "alignment path not found within warp band",
        ));
    }
    Ok((best.0, best.1))
}

fn summarise_segments(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
    path: &[Point],
    segment_frames: usize,
    grid: &[Vec<Cell>],
) -> Result<SegmentSummary> {
    let mut builder = SegmentAccumulator::new(path.len());
    let mut index = 0;
    let mut segment_id = 1;
    while index < path.len() {
        let end = (index + segment_frames).min(path.len());
        builder.push(reference, learner, &path[index..end], segment_id)?;
        index = end;
        segment_id += 1;
    }
    Ok(builder.finish(grid))
}

fn segment_metrics(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
    segment: &[Point],
) -> Result<SegmentMetrics> {
    let mut timing_delta = 0.0;
    let mut similarity = 0.0;
    let mut articulation = 0.0;
    for point in segment {
        timing_delta += frame_delta(point.row, point.col);
        similarity += 1.0 - (point.cost / COST_NORMALISER).min(1.0);
        articulation += flux_delta(reference, learner, point.row, point.col)?;
    }
    let length = segment.len() as f32;
    Ok(SegmentMetrics {
        timing_delta: timing_delta / length,
        similarity: (similarity / length).clamp(0.0, 1.0),
        articulation: (articulation / length).clamp(0.0, 1.0),
        cost: segment.iter().map(|p| p.cost).sum::<f32>() / length,
    })
}

fn build_segment_phoneme(
    segment: &[Point],
    stats: &SegmentMetrics,
    segment_id: usize,
) -> AlignedPhoneme {
    let first = segment.first().unwrap();
    let last = segment.last().unwrap();
    AlignedPhoneme {
        symbol: format!("S{}", segment_id),
        reference_start_ms: frame_to_ms(first.row),
        reference_end_ms: frame_to_ms(last.row + 1),
        learner_start_ms: frame_to_ms(first.col),
        learner_end_ms: frame_to_ms(last.col + 1),
        timing_delta_ms: stats.timing_delta,
        similarity: stats.similarity,
        articulation_variance: stats.articulation,
    }
}

fn frame_delta(reference_frame: usize, learner_frame: usize) -> f32 {
    (learner_frame as i32 - reference_frame as i32) as f32 * FRAME_HOP_MS
}

fn flux_delta(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
    row: usize,
    col: usize,
) -> Result<f32> {
    let ref_flux = reference
        .spectral_flux
        .get(row)
        .ok_or_else(|| PronunciationError::new("spectral flux row out of bounds"))?;
    let learner_flux = learner
        .spectral_flux
        .get(col)
        .ok_or_else(|| PronunciationError::new("spectral flux column out of bounds"))?;
    Ok((ref_flux - learner_flux).abs())
}

fn confidence_from_cost(grid: &[Vec<Cell>]) -> f32 {
    let mut total = 0.0;
    let mut count = 0.0;
    for row in grid {
        for cell in row {
            if cell.local.is_finite() {
                total += cell.local;
                count += 1.0;
            }
        }
    }
    let mean = if count == 0.0 { 0.0 } else { total / count };
    (1.0 / (1.0 + mean / COST_NORMALISER)).clamp(0.0, 1.0)
}

#[derive(Clone, Copy)]
struct Point {
    row: usize,
    col: usize,
    cost: f32,
}

#[derive(Clone, Copy, Default)]
struct Cell {
    cost: f32,
    local: f32,
    direction: Direction,
}

impl Cell {
    fn origin(cost: f32) -> Self {
        Self {
            cost,
            local: cost,
            direction: Direction::Origin,
        }
    }

    fn with_prev(cost: f32, local: f32, direction: Direction) -> Self {
        Self {
            cost,
            local,
            direction,
        }
    }

    fn invalid() -> Self {
        Self {
            cost: f32::INFINITY,
            local: f32::INFINITY,
            direction: Direction::Origin,
        }
    }
}

#[derive(Clone, Copy, Default)]
enum Direction {
    #[default]
    Origin,
    Diagonal,
    Up,
    Left,
}

#[derive(Clone, Copy)]
struct Step {
    cost: f32,
    direction: Direction,
}

impl Step {
    fn new(cost: f32, direction: Direction) -> Self {
        Self { cost, direction }
    }

    fn better(current: Self, cell: Cell, direction: Direction) -> Self {
        if !cell.cost.is_finite() || cell.cost >= current.cost {
            current
        } else {
            Self {
                cost: cell.cost,
                direction,
            }
        }
    }
}

struct SegmentSummary {
    phonemes: Vec<AlignedPhoneme>,
    total_cost: f32,
    global_offset: f32,
    confidence: f32,
    similarity_band: Vec<f32>,
}

struct SegmentMetrics {
    timing_delta: f32,
    similarity: f32,
    articulation: f32,
    cost: f32,
}

struct SegmentAccumulator {
    phonemes: Vec<AlignedPhoneme>,
    similarity: Vec<f32>,
    total_cost: f32,
    total_offset: f32,
    segments: usize,
    path_len: usize,
}

impl SegmentAccumulator {
    fn new(path_len: usize) -> Self {
        Self {
            phonemes: Vec::new(),
            similarity: Vec::new(),
            total_cost: 0.0,
            total_offset: 0.0,
            segments: 0,
            path_len,
        }
    }

    fn push(
        &mut self,
        reference: &PronunciationFeatures,
        learner: &PronunciationFeatures,
        segment: &[Point],
        segment_id: usize,
    ) -> Result<()> {
        let stats = segment_metrics(reference, learner, segment)?;
        self.total_cost += stats.cost;
        self.total_offset += stats.timing_delta;
        self.similarity.push(stats.similarity);
        self.phonemes
            .push(build_segment_phoneme(segment, &stats, segment_id));
        self.segments += 1;
        Ok(())
    }

    fn finish(self, grid: &[Vec<Cell>]) -> SegmentSummary {
        let segments = self.segments.max(1) as f32;
        SegmentSummary {
            phonemes: self.phonemes,
            total_cost: self.total_cost / self.path_len.max(1) as f32,
            global_offset: self.total_offset / segments,
            confidence: confidence_from_cost(grid),
            similarity_band: self.similarity,
        }
    }
}
