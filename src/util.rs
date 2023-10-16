/// Calculate the intersection of two vectors. Returns a tuple of two vectors, the first containing
/// the items that were added, the second containing the items that were removed.
///
/// # Example
/// ```rs
/// let old = vec!["hello", "old"];
/// let new = vec!["hello", "new"];
///
/// let (added, removed) = intersection(&old, &new);
///
/// assert_eq!(added, vec!["new"]);
/// assert_eq!(removed, vec!["old"]);
/// ```
#[inline]
pub fn diff<'a, T: PartialEq>(old: &'a Vec<T>, new: &'a Vec<T>) -> (Vec<&'a T>, Vec<&'a T>) {
    let mut added = Vec::new();
    let mut removed = Vec::new();

    for item in old {
        if new.iter().find(|i| i == &item).is_none() {
            removed.push(item);
        }
    }

    for item in new {
        if old.iter().find(|i| i == &item).is_none() {
            added.push(item);
        }
    }

    (added, removed)
}

#[inline]
pub fn diff_with<'a, T, F>(old: &'a Vec<T>, new: &'a Vec<T>, cmp: F) -> (Vec<&'a T>, Vec<&'a T>)
where
    F: Fn(&T, &T) -> bool,
{
    let mut added = Vec::new();
    let mut removed = Vec::new();

    for item in old {
        if new.iter().find(|i| cmp(i, item)).is_none() {
            removed.push(item);
        }
    }

    for item in new {
        if old.iter().find(|i| cmp(i, item)).is_none() {
            added.push(item);
        }
    }

    (added, removed)
}

#[inline]
pub fn diff_ref_with<'a, T, F>(old: &'a Vec<T>, new: Vec<&'a T>, cmp: F) -> (Vec<&'a T>, Vec<&'a T>)
where
    F: Fn(&T, &T) -> bool,
{
    let mut added = Vec::new();
    let mut removed = Vec::new();

    for item in old {
        if new.iter().find(|i| cmp(i, &*item)).is_none() {
            removed.push(item);
        }
    }

    for item in new {
        if old.iter().find(|i| cmp(i, &*item)).is_none() {
            added.push(item);
        }
    }

    (added, removed)
}
