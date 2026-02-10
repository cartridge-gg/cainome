from __future__ import annotations
from typing import Dict, Iterable, List, Set


def kuhn_max_bipartite_matching(
    left_nodes: Iterable[str],
    adj: Dict[str, list[str]],
) -> Dict[str, str]:
    """
    Kuhn's algorithm (DFS augmenting paths) for maximum bipartite matching.

    Args:
        left_nodes: nodes on the left side.
        right_nodes: nodes on the right side (only used to pre-init structures).
        adj: adjacency list mapping each left node -> iterable of allowed right nodes.

    Returns:
        A dict left->right containing the found matching (maximum size).
        Unmatched left nodes are absent from the dict.
    """
    left_list: List[str] = list(left_nodes)

    # match_r[r] = l means right node r is currently matched to left node l
    match_r: Dict[str, str] = {}

    def try_augment(l: str, seen_left: Set[str]) -> bool:
        """Try to find an augmenting path starting from left node l."""
        
        print(f"try_augment({l})\n")
        
        if l in seen_left:
            print(f"seen\n")
            return False
        
        seen_left.add(l)

        for r in adj.get(l, ()):
            # If r is free, take it
            if r not in match_r:
                print(f"is_free\n")
                match_r[r] = l
                return True

            # Otherwise, try to reroute the current owner of r
            owner = match_r[r]
            print(f"reroute\n")
            if try_augment(owner, seen_left):
                print(f"done\n")
                match_r[r] = l
                return True

        return False

    # Repeatedly try to match each left node
    for l in left_list:
        seen_left: Set[str] = set()
        try_augment(l, seen_left)

    # Invert match_r to get left->right mapping
    match_l: Dict[str, str] = {l: r for r, l in match_r.items()}
    return match_l


def is_perfect_matching(matching: Dict[str, str], left_nodes: Iterable[str]) -> bool:
    left_list = list(left_nodes)
    return len(matching) == len(left_list)


if __name__ == "__main__":
    # Example from your description:
    left = ["f1", "f2", "f3"]
    right = ["a1", "a2", "a3"]

    adj = {
        "f1": ["a3",],
        "f2": ["a2", "a1"],
        "f3": ["a3", "a2"],
    }

    m = kuhn_max_bipartite_matching(left, adj)
    print("matching:", m)
    print("perfect:", is_perfect_matching(m, left))
