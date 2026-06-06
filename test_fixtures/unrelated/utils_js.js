const memo = new Map();

function fibonacci(n) {
    if (memo.has(n)) return memo.get(n);
    if (n <= 1) return n;
    const result = fibonacci(n - 1) + fibonacci(n - 2);
    memo.set(n, result);
    return result;
}

function isPalindrome(str) {
    return str === str.split('').reverse().join('');
}

class TreeNode {
    constructor(val = 0, left = null, right = null) {
        this.val = val;
        this.left = left;
        this.right = right;
    }
}

function treeSum(root) {
    if (root === null) return 0;
    return root.val + treeSum(root.left) + treeSum(root.right);
}
