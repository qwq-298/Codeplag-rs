pub fn bubble_sort(arr: &mut [i32]) {
    let n = arr.len();
    for i in 0..n {
        for j in 0..n - i - 1 {
            if arr[j] > arr[j + 1] {
                arr.swap(j, j + 1);
            }
        }
    }
}

pub fn find_max(arr: &[i32]) -> Option<i32> {
    if arr.is_empty() {
        return None;
    }
    let mut max = arr[0];
    for &item in arr.iter().skip(1) {
        if item > max {
            max = item;
        }
    }
    Some(max)
}
