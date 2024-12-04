use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct Arena<T> {
    nodes: Vec<T>,
}

impl<T> Arena<T> {
    pub fn add(&mut self, node: T) -> ArenaIndex<T> {
        let index = self.nodes.len();
        self.nodes.push(node);
        ArenaIndex::new(index)
    }
    
    pub fn iter(&self) -> impl Iterator<Item = ArenaIndex<T>> + use<'_, T> {
        self.nodes.iter().enumerate().map(|(i, _)| ArenaIndex::new(i))
    }
}

impl<T> Index<ArenaIndex<T>> for Arena<T> {
    type Output = T;

    fn index(&self, index: ArenaIndex<T>) -> &Self::Output {
        &self.nodes[index.index]
    }
}

impl<T> IndexMut<ArenaIndex<T>> for Arena<T> {
    fn index_mut(&mut self, index: ArenaIndex<T>) -> &mut Self::Output {
        &mut self.nodes[index.index]
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::default(),
        }
    }
}

pub struct ArenaIndex<T> {
    index: usize,
    _phantom: PhantomData<T>,
}

impl<T> ArenaIndex<T> {
    const fn new(index: usize) -> Self {
        Self {
            index,
            _phantom: PhantomData,
        }
    }
}

impl<T> Debug for ArenaIndex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("ArenaIndex<{}>", std::any::type_name::<T>())).field(&self.index).finish()
    }
}

impl<T> Clone for ArenaIndex<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for ArenaIndex<T> {}

impl<T> PartialEq for ArenaIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for ArenaIndex<T> {}

impl<T> Hash for ArenaIndex<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> PartialOrd for ArenaIndex<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

impl<T> Ord for ArenaIndex<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}