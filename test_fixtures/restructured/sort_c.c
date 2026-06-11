void bubble_sort_c(int arr[], int n) {
    int swapped;
    do {
        swapped = 0;
        for (int i = 0; i < n - 1; i++) {
            if (arr[i] > arr[i + 1]) {
                int t = arr[i];
                arr[i] = arr[i + 1];
                arr[i + 1] = t;
                swapped = 1;
            }
        }
        n--;
    } while (swapped);
}

int find_max_c(int arr[], int n) {
    if (n <= 0) return -1;
    int m = arr[0];
    for (int i = 0; i < n; i++) {
        if (arr[i] > m) m = arr[i];
    }
    return m;
}
