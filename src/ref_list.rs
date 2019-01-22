
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
enum EntryOrigin {
	Index(usize),
	Detached,
}

impl From<usize> for EntryOrigin {
	fn from(v: usize) -> Self {
		EntryOrigin::Index(v)
	}
}

#[derive(Debug)]
pub struct Entry<T> {
	val: T,
	index: EntryOrigin,
}

impl<T> Entry<T> {
	fn new(val: T, index: usize) -> Entry<T> {
		Entry {
			val: val,
			index: EntryOrigin::Index(index),
		}
	}

	pub fn order(&self) -> Option<usize> {
		match self.index {
			EntryOrigin::Detached => None,
			EntryOrigin::Index(idx) => Some(idx),
		}
	}
}

impl<T> ::std::ops::Deref for Entry<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.val
	}
}

pub struct EntryRef<T>(Rc<RefCell<Entry<T>>>);

impl<T> Clone for EntryRef<T> {
	fn clone(&self) -> Self {
		EntryRef(self.0.clone())
	}
}

impl<T> From<Entry<T>> for EntryRef<T> {
	fn from(v: Entry<T>) -> Self {
		EntryRef(Rc::new(RefCell::new(v)))
	}
}

impl<T> EntryRef<T> {
	fn read(&self) -> ::std::cell::Ref<Entry<T>> {
		self.0.borrow()
	}

	fn write(&self) -> ::std::cell::RefMut<Entry<T>> {
		self.0.borrow_mut()
	}

	fn order(&self) -> Option<usize> {
		self.0.borrow().order()
	}
}

pub struct RefList<T> {
	items: Vec<EntryRef<T>>,
}

impl<T> Default for RefList<T> {
	fn default() -> Self {
		RefList { items: Default::default() }
	}
}

impl<T> RefList<T> {

	pub fn new() -> Self { Self::default() }

	pub fn push(&mut self, t: T) -> EntryRef<T> {
		let idx = self.items.len();
		let val: EntryRef<_> = Entry::new(t, idx).into();
		self.items.push(val.clone());
		val
	}

	pub fn begin_delete(&mut self) -> DeleteTransaction<T> {
		DeleteTransaction {
			list: self,
			deleted: Vec::new(),
		}
	}

	pub fn get(&self, idx: usize) -> Option<EntryRef<T>> {
		self.items.get(idx).cloned()
	}

	fn done_delete(&mut self, indices: &[usize]) {

		let mut index = 0;

		for idx in indices {
			let mut detached = self.items.remove(*idx);
			detached.write().index = EntryOrigin::Detached;
		}

		for index in 0..self.items.len() {
			let mut next_entry = self.items.get_mut(index).expect("Checked above; qed").write();
			let total_less = indices.iter()
				.take_while(|x| **x < next_entry.order().expect("Items in the list always have order; qed"))
				.count();
			match next_entry.index {
				EntryOrigin::Detached => unreachable!("Items in the list always have order!"),
				EntryOrigin::Index(ref mut idx) => { *idx -= total_less; },
			};
		}
	}

	pub fn delete(&mut self, indices: &[usize]) {
		self.done_delete(indices)
	}

	pub fn delete_one(&mut self, index: usize) {
		self.done_delete(&[index])
	}

	pub fn from_slice(list: &[T]) -> Self
		where T: Clone
	{
		let mut res = Self::new();

		for t in list {
			res.push(t.clone());
		}

		res
	}
}

#[must_use]
pub struct DeleteTransaction<'a, T> {
	list: &'a mut RefList<T>,
	deleted: Vec<usize>,
}

impl<'a, T> DeleteTransaction<'a, T> {
	pub fn push(mut self, idx: usize) -> Self {
		let mut tx = self;
		tx.deleted.push(idx);
		tx
	}

	pub fn done(mut self) {
		let indices = self.deleted;
		let list = self.list;
		list.done_delete(&indices[..]);
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn order() {
		let mut list = RefList::<u32>::new();
		let item10 = list.push(10);
		let item20 = list.push(20);
		let item30 = list.push(30);

		assert_eq!(item10.order(), Some(0usize));
		assert_eq!(item20.order(), Some(1));
		assert_eq!(item30.order(), Some(2));

		assert_eq!(**item10.read(), 10);
		assert_eq!(**item20.read(), 20);
		assert_eq!(**item30.read(), 30);
	}

	#[test]
	fn delete() {
		let mut list = RefList::<u32>::new();
		let item10 = list.push(10);
		let item20 = list.push(20);
		let item30 = list.push(30);

		list.begin_delete().push(1).done();

		assert_eq!(item10.order(), Some(0));
		assert_eq!(item30.order(), Some(1));
		assert_eq!(item20.order(), None);
	}
}