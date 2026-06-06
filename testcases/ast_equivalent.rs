// Different syntax, same logic (while instead of for, match instead of if)
pub fn bubble_sort(arr: &mut [i32]) {
    let n = arr.len();
    let mut i = 0;
    while i < n {
        let mut j = 0;
        while j < n - i - 1 {
            // Use match instead of if
            match arr[j] > arr[j + 1] {
                true => arr.swap(j, j + 1),
                false => {}
            }
            j += 1;
        }
        i += 1;
    }
}

pub fn find_max(arr: &[i32]) -> Option<i32> {
    // Use match instead of if
    match arr.len() {
        0 => None,
        _ => {
            let mut max = arr[0];
            let mut idx = 1;
            while idx < arr.len() {
                if arr[idx] > max {
                    max = arr[idx];
                }
                idx += 1;
            }
            Some(max)
        }
    }
}
