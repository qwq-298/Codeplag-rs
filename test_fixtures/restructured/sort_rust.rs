pub fn bubble_sort(arr: &mut [i32]) {
    let n = arr.len();
    let mut swapped = true;
    let mut pass = 0;
    while swapped {
        swapped = false;
        for j in 0..n - pass - 1 {
            if arr[j] > arr[j + 1] {
                arr.swap(j, j + 1);
                swapped = true;
            }
        }
        pass += 1;
    }
}

pub fn find_max(arr: &[i32]) -> Option<i32> {
    arr.iter().max().copied()
}
