public class MySortUtil {
    public static void mySort(int[] data) {
        int size = data.length;
        for (int x = 0; x < size; x++) {
            for (int y = 0; y < size - x - 1; y++) {
                if (data[y] > data[y + 1]) {
                    int tmp = data[y];
                    data[y] = data[y + 1];
                    data[y + 1] = tmp;
                }
            }
        }
    }

    public static int maximumValue(int[] elements) {
        if (elements.length == 0) return -1;
        int biggest = elements[0];
        for (int i = 1; i < elements.length; i++) {
            if (elements[i] > biggest) biggest = elements[i];
        }
        return biggest;
    }
}
