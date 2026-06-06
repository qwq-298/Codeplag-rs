// Same algorithm (bubble sort + find max) but completely rewritten
// — same logic, different implementation patterns
pub fn sort_array(values: &mut [i32]) {
    if values.len() <= 1 {
        return;
    }
    let mut sorted = false;
    let mut end = values.len();
    while !sorted {
        sorted = true;
        for pos in 1..end {
            let prev = values[pos - 1];
            let curr = values[pos];
            if prev > curr {
                values[pos - 1] = curr;
                values[pos] = prev;
                sorted = false;
            }
        }
        end -= 1;
    }
}

pub fn max_element(slice: &[i32]) -> Option<i32> {
    let mut iter = slice.iter();
    let first = iter.next()?;
    let mut best = *first;
    for &val in iter {
        best = best.max(val);
    }
    Some(best)
}
