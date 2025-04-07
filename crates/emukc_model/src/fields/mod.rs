//! Fields Math Prize for Mr.Tanaka

/// Move a value to the end of an array.
pub trait MoveValueToEnd<T>
where
	T: PartialEq,
{
	/// Move a value to the end of an array.
	fn move_value_to_end(&mut self, value: T);
}

impl<T> MoveValueToEnd<T> for Vec<T>
where
	T: PartialEq + Copy,
{
	fn move_value_to_end(&mut self, value: T) {
		// drain_filter is nightly only
		// let removed: Vec<T> = self.drain_filter(|x| *x == value).collect();
		// self.extend(removed);

		let mut remove_count = 0;
		self.retain(|x| {
			if *x != value {
				true
			} else {
				remove_count += 1;
				false
			}
		});

		self.extend(std::iter::repeat_n(value, remove_count));
	}
}

impl<T> MoveValueToEnd<T> for [T]
where
	T: PartialEq + Copy,
{
	fn move_value_to_end(&mut self, value: T) {
		let mut write_index = 0;

		// first pass, move all non-target values to the front
		for read_index in 0..self.len() {
			if self[read_index] != value {
				// 仅当 write_index 和 read_index 不同时才进行赋值
				if write_index != read_index {
					self[write_index] = self[read_index];
				}
				write_index += 1;
			}
		}

		// second pass, fill the rest with target value
		(write_index..self.len()).for_each(|fill_index| {
			self[fill_index] = value;
		});
	}
}

#[cfg(test)]
mod tests {
	use crate::fields::MoveValueToEnd;

	#[test]
	fn test_move_to_end() {
		let mut v = vec![1, -1, 2, -1, 3, -1, 4, -1, 5];
		v.move_value_to_end(-1);
		assert_eq!(v, vec![1, 2, 3, 4, 5, -1, -1, -1, -1]);

		let mut v = [1, -1, 2, -1, 3, -1, 4, -1, 5];
		v.move_value_to_end(-1);
		assert_eq!(v, [1, 2, 3, 4, 5, -1, -1, -1, -1]);

		let mut v = vec![1, 2, 3, 4, 5];
		v.move_value_to_end(-1);
		assert_eq!(v, vec![1, 2, 3, 4, 5]);
	}
}
