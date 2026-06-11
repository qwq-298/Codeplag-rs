public class BubbleSorter {
    public static void bubbleSort(int[] arr) {
        int n = arr.length;
        boolean swapped;
        for (int i = 0; i < n - 1; i++) {
            swapped = false;
            for (int j = 0; j < n - i - 1; j++) {
                if (arr[j] > arr[j + 1]) {
                    int t = arr[j];
                    arr[j] = arr[j + 1];
                    arr[j + 1] = t;
                    swapped = true;
                }
            }
            if (!swapped) break;
        }
    }

    public static int findMax(int[] arr) {
        if (arr == null || arr.length == 0) return -1;
        int max = Integer.MIN_VALUE;
        for (int v : arr) {
            max = Math.max(max, v);
        }
        return max;
    }
}
