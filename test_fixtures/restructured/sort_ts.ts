function bubbleSort(arr: number[]): void {
    let n: number = arr.length;
    let swapped: boolean = true;
    let pass: number = 0;
    while (swapped) {
        swapped = false;
        for (let j = 0; j < n - pass - 1; j++) {
            if (arr[j] > arr[j + 1]) {
                [arr[j], arr[j + 1]] = [arr[j + 1], arr[j]];
                swapped = true;
            }
        }
        pass++;
    }
}

function findMax(arr: number[]): number | null {
    return arr.length === 0 ? null : Math.max(...arr);
}
