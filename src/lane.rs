use crate::span::Span;

/// Greedy swim-lane packing: give each span the lowest-numbered lane whose
/// previous span has already finished. Overlapping work lands on separate rows;
/// sequential work reuses a row. Spans are considered in start-time order, so
/// the result is deterministic for a given input (golden-file friendly).
///
/// Only `Span::lane` is mutated; the slice order is left untouched.
pub fn pack_lanes(spans: &mut [Span]) {
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by_key(|&i| spans[i].start_us); // stable: ties keep input order

    let mut lane_end: Vec<i64> = Vec::new(); // last end time seen on each lane
    for &i in &order {
        let start = spans[i].start_us;
        let end = start + spans[i].dur_us;
        let lane = match lane_end.iter().position(|&e| e <= start) {
            Some(l) => {
                lane_end[l] = end;
                l
            }
            None => {
                lane_end.push(end);
                lane_end.len() - 1
            }
        };
        spans[i].lane = lane as u32;
    }
}
