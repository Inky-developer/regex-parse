use crate::Set;
use std::hash::Hash;

pub trait FloodFill {
    type Item;

    fn get_neighbors(&self, item: &Self::Item) -> impl Iterator<Item = Self::Item>;

    fn iter(&self, start: Self::Item) -> impl Iterator<Item = Self::Item>
    where
        Self: Sized,
        Self::Item: Eq + Hash + Clone,
    {
        let mut pending_nodes = Set::default();
        pending_nodes.insert(start);
        FloodFillIter {
            flood_fill: self,
            pending_nodes,
            visited_nodes: Set::default(),
        }
    }
}

struct FloodFillIter<'a, T: FloodFill> {
    flood_fill: &'a T,
    pending_nodes: Set<T::Item>,
    visited_nodes: Set<T::Item>,
}

impl<T> Iterator for FloodFillIter<'_, T>
where
    T: FloodFill,
    T::Item: Eq + Hash + Clone,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.pending_nodes.iter().next()?;

        let next = next.clone();

        self.visited_nodes.insert(next.clone());
        self.pending_nodes.remove(&next);
        self.pending_nodes.extend(
            self.flood_fill
                .get_neighbors(&next)
                .filter(|neighbor| !self.visited_nodes.contains(neighbor)),
        );

        Some(next)
    }
}
