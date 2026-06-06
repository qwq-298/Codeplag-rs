pub fn my_sort(data: &mut [i32]) {
    let size = data.len();
    for x in 0..size {
        for y in 0..size - x - 1 {
            if data[y] > data[y + 1] {
                data.swap(y, y + 1);
            }
        }
    }
}

pub fn maximum_value(elements: &[i32]) -> Option<i32> {
    if elements.is_empty() {
        return None;
    }
    let mut biggest = elements[0];
    for &e in elements.iter().skip(1) {
        if e > biggest {
            biggest = e;
        }
    }
    Some(biggest)
}
