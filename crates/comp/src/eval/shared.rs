//! A value the evaluator builds once and hands out by reference (a text layout, a mesh).

use std::{ops::Deref, rc::Rc};

/// Two are equal when they are the very same value: the evaluator caches by what builds it, so the same input is the same
/// `Rc`. A cache that was emptied between two frames makes equal things compare different, never different things equal.
#[derive(Debug)]
pub struct Shared<T>(pub Rc<T>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(Rc::clone(&self.0))
    }
}

impl<T> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<Rc<T>> for Shared<T> {
    fn from(rc: Rc<T>) -> Self {
        Shared(rc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_value_is_equal_and_a_copy_of_it_is_not() {
        let a = Shared(Rc::new(5));
        assert!(a == a.clone());
        assert!(a != Shared(Rc::new(5)));
        assert_eq!(*a, 5);
    }
}
