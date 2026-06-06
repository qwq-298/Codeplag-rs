cache = {}

def fibonacci(n):
    if n in cache:
        return cache[n]
    if n <= 1:
        return n
    result = fibonacci(n - 1) + fibonacci(n - 2)
    cache[n] = result
    return result


def is_palindrome(s):
    return s == s[::-1]


class TreeNode:
    def __init__(self, val=0, left=None, right=None):
        self.val = val
        self.left = left
        self.right = right


def tree_sum(root):
    if root is None:
        return 0
    return root.val + tree_sum(root.left) + tree_sum(root.right)
