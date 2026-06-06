function bubbleSort(arr) {
    let n = arr.length;
    for (let i = 0; i < n; i++) {
        for (let j = 0; j < n - i - 1; j++) {
            if (arr[j] > arr[j + 1]) {
                [arr[j], arr[j + 1]] = [arr[j + 1], arr[j]];
            }
        }
    }
    return arr;
}

function findMax(arr) {
    if (arr.length === 0) {
        return null;
    }
    let max = arr[0];
    for (let item of arr.slice(1)) {
        if (item > max) {
            max = item;
        }
    }
    return max;
}
