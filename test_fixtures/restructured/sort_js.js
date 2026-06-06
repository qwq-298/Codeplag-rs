function bubbleSort(arr) {
    let n = arr.length;
    let swapped = true;
    let k = 0;
    while (swapped) {
        swapped = false;
        for (let i = 0; i < n - k - 1; i++) {
            if (arr[i] > arr[i + 1]) {
                [arr[i], arr[i + 1]] = [arr[i + 1], arr[i]];
                swapped = true;
            }
        }
        k++;
    }
    return arr;
}

function findMax(arr) {
    return arr.length === 0 ? null : Math.max(...arr);
}
