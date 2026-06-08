function mySort(data: number[]): void {
    let size: number = data.length;
    for (let x = 0; x < size; x++) {
        for (let y = 0; y < size - x - 1; y++) {
            if (data[y] > data[y + 1]) {
                [data[y], data[y + 1]] = [data[y + 1], data[y]];
            }
        }
    }
}

function maximumValue(elements: number[]): number | null {
    if (elements.length === 0) {
        return null;
    }
    let biggest: number = elements[0];
    for (let i = 1; i < elements.length; i++) {
        if (elements[i] > biggest) {
            biggest = elements[i];
        }
    }
    return biggest;
}
