
/// Partitions the given slice by swapping all elements which select a value GE than split_on to the end of the array.
/// This partitioning operation is unstable.
///
/// Returns the lengths of the left and right sub-partitions.
pub fn partition_by_key<T, K>(slice: &mut [T], split_on: K, selector: impl Fn(&T) -> K) -> (&mut [T], &mut [T])
    where K: PartialOrd
{
    let len = slice.len();
    if len == 0 {
        panic!("partition_by_key: empty input slice");
    }
    let (mut i, mut j) = (0, len - 1);
    while i < j {
        match selector(&slice[i]) {
            v if v <= split_on => {
                // Count into left partition
                i += 1;
            },
            _ => {
                // Swap into right partition
                slice.swap(i, j);
                j -= 1;
            }
        }
    }
    slice.split_at_mut(i + 1)
}

#[cfg(test)]
mod partition_by_key_tests {
    use super::partition_by_key;

    #[test]
    #[should_panic(expected = "partition_by_key: empty input slice")]
    fn test_0() {
        let mut data: [i32; 0] = [];
        let _ = partition_by_key(&mut data, 6, |n| *n);
    }

    #[test]
    fn test_1() {
        let mut data: [i32; 1] = [5];
        let (ll, rl) = partition_by_key(&mut data, 6, |n| *n);
        assert_eq!(ll, &[5]);
        assert_eq!(rl, &[]);
    }

    #[test]
    fn test_2() {
        let mut data: [i32; 6] = [5, 3, 5, 11, 2, 8];
        let (ll, rl) = partition_by_key(&mut data, 6, |n| *n);
        assert_eq!(ll, &[5, 3, 5, 2]);
        assert_eq!(rl, &[8, 11]);
    }

    #[test]
    fn test_3() {
        let mut data: [i32; 3] = [10, 5, 0];
        let (ll, rl) = partition_by_key(&mut data, 5, |n| *n);
        assert_eq!(ll, &[0, 5]);
        assert_eq!(rl, &[10]);
    }
}
