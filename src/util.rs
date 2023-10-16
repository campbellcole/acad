/// Compute the changes between two `Vec`s. Returns a tuple of two vectors, the first
/// containing the items that were added, the second containing the items that were removed.
///
/// # Example
/// ```rs
/// let old = vec!["hello", "old"];
/// let new = vec!["hello", "new"];
///
/// let (added, removed) = intersection(&old, &new, |i1, i2| i1 == i2);
///
/// assert_eq!(added, vec!["new"]);
/// assert_eq!(removed, vec!["old"]);
/// ```
#[inline]
pub fn diff_with<'a, T, F>(old: &'a Vec<T>, new: &'a Vec<T>, cmp: F) -> (Vec<&'a T>, Vec<&'a T>)
where
    F: Fn(&T, &T) -> bool,
{
    // let added = new.into_iter().filter(|i1| {
    //     !old.iter().any(|i2| cmp(i1, i2))
    // }).collect();

    // let removed = old.into_iter().filter(|i1| {
    //     !new.iter().any(|i2| cmp(i1, i2))
    // }).collect();

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

#[cfg(test)]
mod tests {
    use crate::util;

    #[test]
    fn diff_with() {
        let old = vec!["old1", "old2", "both"];
        let new = vec!["new1", "both", "new2"];

        let (added, removed) = util::diff_with(&old, &new, |i1, i2| i1 == i2);

        assert_eq!(added, vec![&"new1", &"new2"]);
        assert_eq!(removed, vec![&"old1", &"old2"]);
    }

    #[test]
    fn diff_ref_with() {
        let old = vec!["old1", "old2", "both"];
        let new = vec!["new1", "both", "new2"];
        let new_refs = new.iter().collect::<Vec<_>>();

        let (added, removed) = util::diff_ref_with(&old, new_refs, |i1, i2| i1 == i2);

        assert_eq!(added, vec![&"new1", &"new2"]);
        assert_eq!(removed, vec![&"old1", &"old2"]);
    }
}
