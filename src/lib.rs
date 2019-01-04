use generational_arena::{Arena, Index};
use std::fmt;

#[derive(PartialEq, Copy, Clone, Debug)]
enum Color {
    RED,
    BLACK,
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum NodeType {
    LeftChild,
    RightChild,
    Orphan,
}

#[derive(Debug)]
pub struct Node<T: Clone + fmt::Debug> {
    parent: Option<Index>,
    left: Option<Index>,
    right: Option<Index>,

    contents: T,

    prev: Option<Index>,
    next: Option<Index>,

    color: Color,
}

impl<T: Clone + fmt::Debug> Node<T> {
    fn new(contents: T) -> Self {
        Node {
            // Tree structure
            parent: None,
            left: None,
            right: None,

            // Data
            contents,

            // Optimisation
            prev: None,
            next: None,
            color: Color::RED,
        }
    }
}

/// The tree structure.
/// Stores the nodes in a genrational arena and the index of the root of the tree.
pub struct Tree<T: Clone + fmt::Debug> {
    nodes: Arena<Node<T>>,
    pub root: Option<Index>,
}

impl<T: Clone + fmt::Debug> Tree<T> {
    /// Create a new empty tree
    pub fn new() -> Self {
        Tree {
            nodes: Arena::new(),
            root: None,
        }
    }

    /// Utility functon to check if the tree has a root node or not
    pub fn has_root(&self) -> bool {
        self.root.is_some()
    }

    /// Creates a new root node for the tree and returns the index of the created node.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to populate the new node with
    ///
    pub fn create_root(&mut self, value: T) -> Index {
        debug_assert!(!self.has_root());
        let root = self.nodes.insert(Node::new(value));
        self.set_color(root, Color::BLACK);
        self.root = Some(root);
        root
    }

    /// Create and insert a new node immediately after the specified node and rebalance the tree
    /// Returns the index of the newly created node.
    ///
    /// # Arguments
    ///
    /// * `existing_node` - The index of the existing node to insert the new node before
    /// * `value` - The value to populate the newly created node with
    ///
    pub fn insert_after(&mut self, existing_node: Index, value: T) -> Index {
        let new_node = self.nodes.insert(Node::new(value));
        let existing_node_next = self.get_next(existing_node);
        if self.get_right(existing_node).is_none() {
            self.set_right(existing_node, Some(new_node));
            self.set_parent(new_node, Some(existing_node));
        } else {
            self.set_left(existing_node_next.unwrap(), Some(new_node));
            self.set_parent(new_node, existing_node_next);
        }

        self.set_next(new_node, existing_node_next);
        match self.get_next(new_node) {
            Some(next) => self.set_prev(next, Some(new_node)),
            None => {}
        }
        self.set_prev(new_node, Some(existing_node));
        self.set_next(existing_node, Some(new_node));

        // Balance the tree
        self.insert_rebalance(new_node);

        new_node
    }

    /// Create and insert a new node immediately before the specified node and rebalance the tree.
    /// Returns the index of the newly created node.
    ///
    /// # Arguments
    ///
    /// * `existing_node` - The index of the existing node to insert the new node before
    /// * `value` - The value to populate the newly created node with
    ///
    pub fn insert_before(&mut self, existing_node: Index, value: T) -> Index {
        let new_node = self.nodes.insert(Node::new(value));
        let existing_node_prev = self.get_prev(existing_node);
        if self.get_left(existing_node).is_none() {
            self.set_left(existing_node, Some(new_node));
            self.set_parent(new_node, Some(existing_node));
        } else {
            self.set_right(existing_node_prev.unwrap(), Some(new_node));
            self.set_parent(new_node, existing_node_prev);
        }
        self.set_prev(new_node, existing_node_prev);
        if existing_node_prev.is_some() {
            self.set_next(existing_node_prev.unwrap(), Some(new_node));
        }
        self.set_next(new_node, Some(existing_node));
        self.set_prev(existing_node, Some(new_node));

        // Balance the tree
        self.insert_rebalance(new_node);

        new_node
    }

    /// Delete the specified node from the tree and rebalance the remaining nodes
    ///
    /// # Arguments
    ///
    /// * `node` - The index of the node to delete from the tree
    ///
    pub fn delete_node(&mut self, node: Index) {
        if self.get_left(node).is_some() && self.get_right(node).is_some() {
            self.swap_nodes(node, self.get_next(node).unwrap());
        }

        let replacement = self.get_replacement_node(node);
        let both_black = self.get_color(Some(node)) == Color::BLACK
            && self.get_color(replacement) == Color::BLACK;
        if replacement.is_none() {
            // The node is a leaf
            if Some(node) == self.root {
                // node is the root so set the root to None
                self.root = None;
            } else {
                if both_black {
                    // Both the node and the replacement are black
                    // As v is a leaf we need to fix the double black at v
                    self.fix_double_black(node);
                } else {
                    // The node must be red
                    let sibling = self.get_sibling(node);
                    if sibling.is_some() {
                        self.set_color(sibling.unwrap(), Color::RED);
                    }
                }
                let parent = self.get_parent(node);
                match self.get_node_type(node) {
                    NodeType::LeftChild => self.set_left(parent.unwrap(), None),
                    NodeType::RightChild => self.set_right(parent.unwrap(), None),
                    NodeType::Orphan => panic!("None root node can't be an orphan"),
                }
            }
            self.update_order_for_deletion(node);
            self.nodes.remove(node);
        } else {
            if Some(node) == self.root {
                // Removing the root node
                self.swap_nodes(node, replacement.unwrap());
                self.set_left(replacement.unwrap(), None);
                self.set_right(replacement.unwrap(), None);
                self.nodes.remove(node);
            } else {
                let parent = self.get_parent(node);
                match self.get_node_type(node) {
                    NodeType::LeftChild => self.set_left(parent.unwrap(), replacement),
                    NodeType::RightChild => self.set_right(parent.unwrap(), replacement),
                    NodeType::Orphan => panic!("None root node can't be an orphan"),
                }
                if replacement.is_some() {
                    self.set_parent(replacement.unwrap(), parent);
                }
                self.update_order_for_deletion(node);
                self.nodes.remove(node);
                if both_black {
                    self.fix_double_black(node);
                } else {
                    self.set_color(replacement.unwrap(), Color::BLACK);
                }
            }
        }
    }

    // Finds the node that will replace a deleted node in the tree
    fn get_replacement_node(&self, node: Index) -> Option<Index> {
        let left = self.get_left(node);
        let right = self.get_right(node);
        if left.is_some() && right.is_some() {
            // Node has two children we can't replace it
            panic!("Cannot find replacement node for a node with two child nodes");
        } else if left.is_none() && right.is_none() {
            // Node is a leaf
            None
        } else if left.is_some() {
            // Single Child on left
            left
        } else {
            // Single Child on right
            right
        }
    }

    // Updates the prev and next entrys of a node that is being deleted to ensure that the order of the nodes is correct
    fn update_order_for_deletion(&mut self, deleted_node: Index) {
        let next = self.get_next(deleted_node);
        let prev = self.get_prev(deleted_node);
        if next.is_some() {
            self.set_prev(next.unwrap(), prev);
        }
        if prev.is_some() {
            self.set_next(prev.unwrap(), next);
        }
    }

    // Fix a double black node that has been caused by deleting a node
    fn fix_double_black(&mut self, mut node: Index) {
        while Some(node) != self.root {
            let sibling = self.get_sibling(node);
            let parent = self.get_parent(node);
            if sibling.is_none() {
                // No sibling, double black is pushed up
                node = parent.unwrap()
            } else {
                if self.get_color(sibling) == Color::RED {
                    // Sibling is red
                    self.set_color(parent.unwrap(), Color::RED);
                    self.set_color(sibling.unwrap(), Color::BLACK);
                    match self.get_node_type(sibling.unwrap()) {
                        NodeType::LeftChild => self.right_rotate(parent.unwrap()),
                        NodeType::RightChild => self.left_rotate(parent.unwrap()),
                        NodeType::Orphan => panic!("A sibling node can't be an orphan"),
                    }
                } else {
                    if self.get_color(self.get_left(sibling.unwrap())) == Color::RED {
                        let left = self.get_left(sibling.unwrap()).unwrap();
                        match self.get_node_type(sibling.unwrap()) {
                            NodeType::LeftChild => {
                                // Left-Left
                                self.set_color(left, self.get_color(sibling));
                                self.set_color(sibling.unwrap(), self.get_color(parent));
                                self.right_rotate(parent.unwrap());
                            }
                            NodeType::RightChild => {
                                // Right-Left
                                self.set_color(left, self.get_color(parent));
                                self.right_rotate(sibling.unwrap());
                                self.left_rotate(parent.unwrap());
                            }
                            NodeType::Orphan => panic!("A sibling node can't be an orphan"),
                        }
                        self.set_color(parent.unwrap(), Color::BLACK);
                        break;
                    } else if self.get_color(self.get_right(sibling.unwrap())) == Color::RED {
                        let right = self.get_right(sibling.unwrap()).unwrap();
                        match self.get_node_type(sibling.unwrap()) {
                            NodeType::LeftChild => {
                                // Left-Right
                                self.set_color(right, self.get_color(parent));
                                self.left_rotate(sibling.unwrap());
                                self.right_rotate(parent.unwrap());
                            }
                            NodeType::RightChild => {
                                // Right-Right
                                self.set_color(right, self.get_color(sibling));
                                self.set_color(sibling.unwrap(), self.get_color(parent));
                                self.left_rotate(parent.unwrap());
                            }
                            NodeType::Orphan => panic!("A sibling node can't be an orphan"),
                        }
                        self.set_color(parent.unwrap(), Color::BLACK);
                        break;
                    } else {
                        // 2 Black children
                        self.set_color(sibling.unwrap(), Color::RED);
                        if self.get_color(parent) == Color::BLACK {
                            node = parent.unwrap();
                        } else {
                            self.set_color(parent.unwrap(), Color::BLACK);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Rebalances the tree after inserting a new node
    fn insert_rebalance(&mut self, mut node: Index) {
        while self.get_color(self.get_parent(node)) == Color::RED {
            // Only get here for cases 3, 4 and 5, cases 1 and 2 are trivial
            // Parent is RED so it exists
            let mut parent = self.get_parent(node).unwrap();
            // As parent is red it isn't the root so can get a grandparent
            let grandparent = self.get_parent(parent).unwrap();
            let uncle = self.get_uncle(node);
            if self.get_color(uncle) == Color::RED {
                // Uncle is red so we can balance by re-coloring parent and uncle red
                self.set_color(uncle.unwrap(), Color::BLACK);
                // Uncle is red so can safely unwrap
                self.set_color(parent, Color::BLACK);
                // Set grandparent to red and recurse up up the tree
                self.set_color(grandparent, Color::RED);
                node = grandparent;
            } else {
                // Uncle is black We will need to do some rotations
                let parent_node_type = self.get_node_type(parent);
                if self.get_node_type(node) != parent_node_type {
                    //  Left-right or right-left case,
                    if parent_node_type == NodeType::LeftChild {
                        self.left_rotate(parent);
                    } else {
                        self.right_rotate(parent);
                    }
                    // We've swapped the parent and node so account for this
                    node = parent;
                    parent = self.get_parent(node).unwrap();
                }
                // Left-Left or Right-Right Case
                // Uncle is black so we will need to rotate the grandparent away from the conflict(to the right)
                self.set_color(parent, Color::BLACK);
                self.set_color(grandparent, Color::RED);
                if self.get_node_type(parent) == NodeType::LeftChild {
                    self.right_rotate(grandparent);
                } else {
                    self.left_rotate(grandparent);
                }
            }
        }
        self.set_color(self.root.unwrap(), Color::BLACK);
    }

    // Roates the nodes to the left
    //    p              q
    //   / \            / \
    //  a   q   -->    p   c
    //     / \        / \
    //    b   c      a   b
    fn left_rotate(&mut self, rotation_root: Index) {
        // Left rotation so pivot is to the right
        let pivot = self.get_right(rotation_root).unwrap();
        let pivot_left = self.get_left(pivot);
        let parent = self.get_parent(rotation_root);
        // The left child of the pivot becomes the right child of the rotation root
        self.set_right(rotation_root, pivot_left);
        if pivot_left.is_some() {
            self.set_parent(pivot_left.unwrap(), Some(rotation_root));
        }

        // The pivot replaces the rotation root in the tree
        self.set_parent(pivot, parent);
        match self.get_node_type(rotation_root) {
            NodeType::LeftChild => self.set_left(parent.unwrap(), Some(pivot)),
            NodeType::RightChild => self.set_right(parent.unwrap(), Some(pivot)),
            NodeType::Orphan => self.root = Some(pivot),
        }

        // Set the left child of the pivot to be the rotation root
        self.set_left(pivot, Some(rotation_root));
        self.set_parent(rotation_root, Some(pivot));
    }

    // Rotates the nodes to the right
    //     q             p
    //    / \           / \
    //   p   c  -->    a   q
    //  / \               / \
    // a   b             b   c
    fn right_rotate(&mut self, rotation_root: Index) {
        // Right rotation so pivot is to the left
        let pivot = self.get_left(rotation_root).unwrap();
        let pivot_right = self.get_right(pivot);
        let parent = self.get_parent(rotation_root);
        // The right child of the pivot becomes the left child of the rotation root
        self.set_left(rotation_root, pivot_right);
        if pivot_right.is_some() {
            self.set_parent(pivot_right.unwrap(), Some(rotation_root));
        }

        // The pivot replaces the rotation root in the tree
        self.set_parent(pivot, parent);
        match self.get_node_type(rotation_root) {
            NodeType::LeftChild => self.set_left(parent.unwrap(), Some(pivot)),
            NodeType::RightChild => self.set_right(parent.unwrap(), Some(pivot)),
            NodeType::Orphan => self.root = Some(pivot),
        }

        // Set the right child of the pivot to be the rotation root
        self.set_right(pivot, Some(rotation_root));
        self.set_parent(rotation_root, Some(pivot));
    }

    // Swap the location in the tree of two nodes
    fn swap_nodes(&mut self, node_1: Index, node_2: Index) {
        let mut node_1_parent = self.get_parent(node_1);
        let mut node_2_parent = self.get_parent(node_2);
        let node_1_right = self.get_right(node_1);
        let node_2_right = self.get_right(node_2);
        let node_1_left = self.get_left(node_1);
        let node_2_left = self.get_left(node_2);
        // Swap parents
        if node_1_parent == Some(node_2) {
            node_1_parent = Some(node_1);
        } else if node_2_parent == Some(node_1) {
            node_2_parent = Some(node_2);
        }
        match self.get_node_type(node_1) {
            NodeType::LeftChild => self.set_left(node_1_parent.unwrap(), Some(node_2)),
            NodeType::RightChild => self.set_right(node_1_parent.unwrap(), Some(node_2)),
            NodeType::Orphan => self.root = Some(node_2),
        };
        match self.get_node_type(node_2) {
            NodeType::LeftChild => self.set_left(node_2_parent.unwrap(), Some(node_1)),
            NodeType::RightChild => self.set_right(node_2_parent.unwrap(), Some(node_1)),
            NodeType::Orphan => self.root = Some(node_1),
        };
        self.set_parent(node_1, node_2_parent);
        self.set_parent(node_2, node_1_parent);

        // Swap Left Children
        if node_2_left != Some(node_1) {
            self.set_left(node_1, node_2_left);
            if node_2_left.is_some() {
                self.set_parent(node_2_left.unwrap(), Some(node_1));
            }
        }
        if node_1_left != Some(node_2) {
            self.set_left(node_2, node_1_left);
            if node_1_left.is_some() {
                self.set_parent(node_1_left.unwrap(), Some(node_2));
            }
        }

        // Swap Right Children
        if node_2_right != Some(node_1) {
            self.set_right(node_1, node_2_right);
            if node_2_right.is_some() {
                self.set_parent(node_2_right.unwrap(), Some(node_1));
            }
        }
        if node_1_right != Some(node_2) {
            self.set_right(node_2, node_1_right);
            if node_1_right.is_some() {
                self.set_parent(node_1_right.unwrap(), Some(node_2));
            }
        }

        // Swap Colors
        let node_1_color = self.get_color(Some(node_1));
        self.set_color(node_1, self.get_color(Some(node_2)));
        self.set_color(node_2, node_1_color);
    }

    // Returns a NodeType enum indicating if the given node is a left child, right child in
    // relation to it's parent or an orphan
    fn get_node_type(&self, node: Index) -> NodeType {
        let parent = self.get_parent(node);
        if parent.is_some() {
            if self.get_left(parent.unwrap()) == Some(node) {
                NodeType::LeftChild
            } else {
                NodeType::RightChild
            }
        } else {
            // Not a child node at all
            NodeType::Orphan
        }
    }

    ///  Returns the sibling node to the current node, that is the other node that shares the same parent
    fn get_sibling(&self, node: Index) -> Option<Index> {
        let parent = self.get_parent(node);
        match self.get_node_type(node) {
            NodeType::LeftChild => self.get_right(parent.unwrap()),
            NodeType::RightChild => self.get_left(parent.unwrap()),
            NodeType::Orphan => None,
        }
    }

    // Returns the uncle node of the current node, that is the sibling of the parent node if it exists.
    fn get_uncle(&self, node: Index) -> Option<Index> {
        let parent = self.get_parent(node);
        if parent.is_some() {
            match self.get_node_type(parent.unwrap()) {
                NodeType::LeftChild => self.get_right(self.get_parent(parent.unwrap()).unwrap()),
                NodeType::RightChild => self.get_left(self.get_parent(parent.unwrap()).unwrap()),
                NodeType::Orphan => None,
            }
        } else {
            None
        }
    }

    // Getter and setters
    fn set_right(&mut self, node: Index, right: Option<Index>) {
        let node = self.nodes.get_mut(node).unwrap();
        node.right = right;
    }

    fn get_right(&self, node: Index) -> Option<Index> {
        let node = self.nodes.get(node).unwrap();
        node.right
    }

    fn set_left(&mut self, node: Index, left: Option<Index>) {
        let node = self.nodes.get_mut(node).unwrap();
        node.left = left;
    }

    fn get_left(&self, node: Index) -> Option<Index> {
        let node = self.nodes.get(node).unwrap();
        node.left
    }

    fn set_parent(&mut self, node: Index, parent: Option<Index>) {
        let node = self.nodes.get_mut(node).unwrap();
        node.parent = parent;
    }

    fn get_parent(&self, node: Index) -> Option<Index> {
        let node = self.nodes.get(node).unwrap();
        node.parent
    }

    fn set_prev(&mut self, node: Index, prev: Option<Index>) {
        let node = self.nodes.get_mut(node).unwrap();
        node.prev = prev;
    }

    fn get_prev(&self, node: Index) -> Option<Index> {
        let node = self.nodes.get(node).unwrap();
        node.prev
    }

    fn set_next(&mut self, node: Index, next: Option<Index>) {
        let node = self.nodes.get_mut(node).unwrap();
        node.next = next;
    }

    fn get_next(&self, node: Index) -> Option<Index> {
        let node = self.nodes.get(node).unwrap();
        node.next
    }

    fn set_color(&mut self, node: Index, color: Color) {
        let node = self.nodes.get_mut(node).unwrap();
        node.color = color;
    }

    fn get_color(&self, node: Option<Index>) -> Color {
        if node.is_none() {
            Color::BLACK
        } else {
            match self.nodes.get(node.unwrap()) {
                Some(node) => node.color,
                None => Color::BLACK,
            }
        }
    }

    /// Set the contents of the specified
    ///
    /// # Arguments
    ///
    /// * `node` - The node to set the contents on
    /// * `contents` - The new contents to populate the node with
    ///
    pub fn set_contents(&mut self, node: Index, contents: T) {
        let node = self.nodes.get_mut(node).unwrap();
        node.contents = contents;
    }

    /// Returns a refernence to the contents of the specified node
    ///
    /// # Arguments
    ///
    /// * `node` - The node to return the contents of
    ///
    pub fn get_contents(&self, node: Index) -> &T {
        let node = self.nodes.get(node).unwrap();
        &node.contents
    }

    /// Returns a mutable refernence to the contents of the specified node
    ///
    /// # Arguments
    ///
    /// * `node` - The node to return the contents of
    ///
    pub fn get_mut_contents(&mut self, node: Index) -> &mut T {
        let node = self.nodes.get_mut(node).unwrap();
        &mut node.contents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<T: Clone + fmt::Debug> Tree<T> {
        fn check_black_heights(&self, node: Option<Index>) -> usize {
            if node.is_none() {
                1
            } else {
                let left_height = self.check_black_heights(self.get_left(node.unwrap()));
                let right_height = self.check_black_heights(self.get_right(node.unwrap()));
                if left_height != right_height {
                    panic!(
                        "Invalid black height for node at {:?}",
                        self.get_contents(node.unwrap())
                    )
                }
                if self.get_color(node) == Color::RED {
                    left_height
                } else {
                    left_height + 1
                }
            }
        }

        pub fn get_level_order(&self) -> String {
            let mut out = "".to_string();
            if self.root.is_some() {
                let mut queue = vec![self.root.unwrap()];
                let mut current_node: Option<&Index>;

                while !queue.is_empty() {
                    current_node = queue.first();

                    out = format!("{}{:?} ", &out, self.get_contents(*current_node.unwrap()));

                    let left = self.get_left(*current_node.unwrap());
                    let right = self.get_right(*current_node.unwrap());
                    if left.is_some() {
                        queue.push(left.unwrap());
                    }
                    if right.is_some() {
                        queue.push(right.unwrap());
                    }

                    queue.remove(0);
                }
            }
            out
        }

        fn get_leftmost_node(&self) -> Option<Index> {
            let mut node = self.root;
            if node.is_some() {
                while self.get_left(node.unwrap()).is_some() {
                    node = self.get_left(node.unwrap());
                }
            }
            node
        }

        pub fn get_nodes_order(&self) -> String {
            let mut out = "".to_string();
            let mut node = self.get_leftmost_node();
            while node.is_some() {
                out = format!("{}{:?} ", out, self.get_contents(node.unwrap()));
                node = self.get_next(node.unwrap());
            }
            out
        }
    }

    #[test]
    fn insertion_test() {
        let mut tree: Tree<usize> = Tree::new();

        let seven = tree.create_root(7);
        assert_eq!(tree.check_black_heights(tree.root), 2);
        assert_eq!(tree.get_level_order(), "7 ");
        assert_eq!(tree.get_nodes_order(), "7 ");

        let six = tree.insert_before(seven, 6);
        assert_eq!(tree.check_black_heights(tree.root), 2);
        assert_eq!(tree.get_level_order(), "7 6 ");
        assert_eq!(tree.get_nodes_order(), "6 7 ");

        let five = tree.insert_before(six, 5);
        assert_eq!(tree.check_black_heights(tree.root), 2);
        assert_eq!(tree.get_level_order(), "6 5 7 ");
        assert_eq!(tree.get_nodes_order(), "5 6 7 ");

        let four = tree.insert_before(five, 4);
        assert_eq!(tree.check_black_heights(tree.root), 3);
        assert_eq!(tree.get_level_order(), "6 5 7 4 ");
        assert_eq!(tree.get_nodes_order(), "4 5 6 7 ");

        let three = tree.insert_before(four, 3);
        assert_eq!(tree.check_black_heights(tree.root), 3);
        assert_eq!(tree.get_level_order(), "6 4 7 3 5 ");
        assert_eq!(tree.get_nodes_order(), "3 4 5 6 7 ");

        let two = tree.insert_before(three, 2);
        assert_eq!(tree.check_black_heights(tree.root), 3);
        assert_eq!(tree.get_level_order(), "6 4 7 3 5 2 ");
        assert_eq!(tree.get_nodes_order(), "2 3 4 5 6 7 ");

        let _one = tree.insert_before(two, 1);
        assert_eq!(tree.get_level_order(), "6 4 7 2 5 1 3 ");
        assert_eq!(tree.get_nodes_order(), "1 2 3 4 5 6 7 ");

        assert_eq!(tree.check_black_heights(tree.root), 3);
    }

    #[test]
    fn deletion_test() {
        let mut tree: Tree<usize> = Tree::new();

        let seven = tree.create_root(7);

        let three = tree.insert_before(seven, 3);
        let eighteen = tree.insert_after(seven, 18);
        let ten = tree.insert_after(seven, 10);
        let twentytwo = tree.insert_after(eighteen, 22);
        let _eight = tree.insert_before(ten, 8);
        let eleven = tree.insert_after(ten, 11);
        let _twentysix = tree.insert_after(twentytwo, 26);
        let _two = tree.insert_before(three, 2);
        let _six = tree.insert_before(seven, 6);
        let _thirteen = tree.insert_after(eleven, 13);

        assert_eq!(tree.get_level_order(), "10 7 18 3 8 11 22 2 6 13 26 ");
        assert_eq!(tree.get_nodes_order(), "2 3 6 7 8 10 11 13 18 22 26 ");
        assert_eq!(tree.check_black_heights(tree.root), 3);

        tree.delete_node(eighteen);
        assert_eq!(tree.get_level_order(), "10 7 22 3 8 11 26 2 6 13 ");
        assert_eq!(tree.get_nodes_order(), "2 3 6 7 8 10 11 13 22 26 ");
        tree.delete_node(eleven);
        assert_eq!(tree.get_level_order(), "10 7 22 3 8 13 26 2 6 ");
        assert_eq!(tree.get_nodes_order(), "2 3 6 7 8 10 13 22 26 ");
        tree.delete_node(three);
        assert_eq!(tree.get_level_order(), "10 7 22 6 8 13 26 2 ");
        assert_eq!(tree.get_nodes_order(), "2 6 7 8 10 13 22 26 ");
        tree.delete_node(ten);
        assert_eq!(tree.get_level_order(), "13 7 22 6 8 26 2 ");
        assert_eq!(tree.get_nodes_order(), "2 6 7 8 13 22 26 ");
        tree.delete_node(twentytwo);
        assert_eq!(tree.get_level_order(), "13 7 26 6 8 2 ");
        assert_eq!(tree.get_nodes_order(), "2 6 7 8 13 26 ");

        assert_eq!(tree.check_black_heights(tree.root), 3);
    }
}
