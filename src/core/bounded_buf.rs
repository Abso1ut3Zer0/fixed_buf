use std::{
    alloc::{alloc, dealloc, Layout},
    isize,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

#[derive(Debug, Clone)]
pub struct BoundedBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
}

impl<T> BoundedBuffer<T> {
    pub fn new(size: usize) -> Self {
        assert!(size <= isize::MAX as usize, "size is too large");
        unsafe {
            let layout = Layout::array::<T>(size).unwrap_unchecked();
            let ptr = alloc(layout);

            assert!(!ptr.is_null(), "could not allocate");
            Self {
                ptr: NonNull::new_unchecked(ptr as *mut T),
                len: 0,
                cap: size,
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index > self.len {
            return None;
        }

        unsafe { Some(self.ptr.add(index).as_ref()) }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    pub fn as_mut_slice(&self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    pub fn try_push(&mut self, elem: T) -> bool {
        if self.len == self.cap {
            return false;
        }

        unsafe {
            self.push_unchecked(elem);
        }
        true
    }

    pub fn try_insert(&mut self, index: usize, elem: T) -> bool {
        if self.len == self.cap || index > self.len {
            return false;
        }

        unsafe {
            self.insert_unchecked(index, elem);
        }
        true
    }

    pub fn insert_lossy(&mut self, index: usize, elem: T) {
        unsafe {
            std::ptr::copy(
                self.ptr.add(index).as_ptr(),
                self.ptr.add(index + 1).as_ptr(),
                self.len() - index,
            );
            self.ptr.add(index).write(elem);
            self.len = usize::min(self.cap, self.len + 1);
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");
        self.len -= 1;
        unsafe {
            let val = std::ptr::read(self.ptr.add(index).as_ptr());
            std::ptr::copy(
                self.ptr.add(index + 1).as_ptr(),
                self.ptr.add(index).as_ptr(),
                self.len - index,
            );
            val
        }
    }

    pub fn clear(&mut self) {
        let slice = self.as_mut_slice();
        unsafe {
            std::ptr::drop_in_place(slice);
            self.len = 0;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        unsafe { Some(std::ptr::read(self.ptr.add(self.len).as_ptr())) }
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        self.ptr.add(index).as_ref()
    }

    pub unsafe fn push_unchecked(&mut self, elem: T) {
        self.ptr.add(self.len).write(elem);
        self.len += 1;
    }

    pub unsafe fn insert_unchecked(&mut self, index: usize, elem: T) {
        std::ptr::copy(
            self.ptr.add(index).as_ptr(),
            self.ptr.add(index + 1).as_ptr(),
            self.len() - index,
        );
        self.ptr.add(index).write(elem);
        self.len += 1;
    }
}

impl<T> Drop for BoundedBuffer<T> {
    fn drop(&mut self) {
        let elem_size = std::mem::size_of::<T>();
        if self.cap != 0 && elem_size != 0 {
            unsafe {
                dealloc(
                    self.ptr.as_ptr() as *mut u8,
                    Layout::array::<T>(self.cap).unwrap_unchecked(),
                );
            }
        }
    }
}

impl<T> Deref for BoundedBuffer<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for BoundedBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> AsRef<[T]> for BoundedBuffer<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for BoundedBuffer<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> AsRef<BoundedBuffer<T>> for BoundedBuffer<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T> AsMut<BoundedBuffer<T>> for BoundedBuffer<T> {
    fn as_mut(&mut self) -> &mut BoundedBuffer<T> {
        self
    }
}

#[cfg(test)]
mod test {
    use super::BoundedBuffer;

    #[test]
    fn test_bounded_array() {
        let n = 10_000;
        let mut arr1 = BoundedBuffer::new(n);

        let now = std::time::Instant::now();
        for i in 0..n {
            arr1.try_push(i);
        }
        let elapsed = now.elapsed().as_nanos();
        println!("BoundedBuffer Len: {}", arr1.len());
        println!("BoundedBuffer Push: {} ns/op", elapsed / n as u128);

        let mut arr2 = Vec::with_capacity(n);

        let now = std::time::Instant::now();
        for i in 0..n {
            arr2.push(i);
        }

        let elapsed = now.elapsed().as_nanos();
        println!("Vec Len: {}", arr2.len());
        println!("Vec Push: {} ns/op", elapsed / n as u128);

        for i in 0..n {
            let val1 = arr1.get(i);
            let val2 = arr2.get(i);
            assert!(val1.is_some());
            assert!(val2.is_some());

            let val1 = val1.unwrap();
            let val2 = val2.unwrap();

            assert_eq!(val1, val2);
        }

        let slice = arr1.as_slice();
        for i in 0..n {
            let val1 = slice[i];
            let val2 = arr2[i];
            assert_eq!(val1, val2);
        }

        for (val1, val2) in arr1.iter().zip(arr2.iter()) {
            assert_eq!(val1, val2);
        }

        let mut sum = 0;
        let now = std::time::Instant::now();
        for i in 0..n {
            sum += arr1.get(i).unwrap();
        }
        let elapsed = now.elapsed().as_nanos();
        println!("BoundedBuffer Get: {} ns/op", elapsed / n as u128);
        println!("Sum: {}", sum);

        let mut sum = 0;
        let now = std::time::Instant::now();
        for i in 0..n {
            sum += arr2.get(i).unwrap();
        }

        let elapsed = now.elapsed().as_nanos();
        println!("Vec Get: {} ns/op", elapsed / n as u128);
        println!("Sum: {}", sum);

        arr1.clear();
        arr2.clear();

        let now = std::time::Instant::now();
        for i in 0..n {
            arr1.try_insert(0, i);
        }
        let elapsed = now.elapsed().as_nanos();
        println!("BoundedBuffer Try Insert: {} ns/op", elapsed / n as u128);

        let now = std::time::Instant::now();
        for i in 0..n {
            arr2.insert(0, i);
        }
        let elapsed = now.elapsed().as_nanos();
        println!("Vec Insert: {} ns/op", elapsed / n as u128);
    }
}
