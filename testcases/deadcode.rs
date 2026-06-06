// Copied code with extra dead functions and unused variables added
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

// Dead code — should not affect similarity of the real functions
pub fn unused_helper() -> i32 {
    let x = 42;
    let y = x * 2;
    y + 10
}

pub fn deprecated_sort(v: &mut Vec<i32>) {
    let _unused = "this function is never called";
    for _ in 0..v.len() {
        // do nothing
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
