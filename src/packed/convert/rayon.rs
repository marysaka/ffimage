use std::cell::UnsafeCell;
use std::ops::Index;

use num_traits::identities::Zero;

use rayon::prelude::*;

use crate::core::traits::{GenericImageView, Pixel, Convert};
use crate::packed::traits::ConvertSlice;
use crate::packed::generic::{ImageViewMut, ImageBuffer};

// This is a private helper struct to share buffers between threads in a lock free manner where we
// would usually need a Mutex. Only use this when you can ensure that all usage of the wrapped
// value is safe and never interleaved!
struct UnsafeShared<T: ?Sized> {
    value: UnsafeCell<T>,
}

impl<T> UnsafeShared<T> {
    pub fn new(t: T) -> UnsafeShared<T> {
        UnsafeShared {
            value: UnsafeCell::new(t),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get(&self) -> &mut T {
        &mut *self.value.get()
    }
}

unsafe impl<T: ?Sized + Send> Send for UnsafeShared<T> {}
unsafe impl<T: ?Sized + Send> Sync for UnsafeShared<T> {}

impl <'a, 'b, DP, I> Convert<ImageViewMut<'b, DP>> for I
where
    DP: Pixel,
    DP::T: Send,
    I: GenericImageView<'a> + Index<usize> + Sync,
    <I as Index<usize>>::Output: Index<usize>,
    <I as Index<usize>>::Output: AsRef<[<<I as Index<usize>>::Output as Index<usize>>::Output]>,
    <<I as Index<usize>>::Output as Index<usize>>::Output: Pixel + ConvertSlice<DP>,

{
    fn convert(&self, output: &mut ImageViewMut<'b, DP>) {
        let row_count = if output.height() < self.height() {
            output.height()
        } else {
            self.height()
        };

        // It is safe to use the shared, lock free wrapper here because each thread
        // accesses a distinct pixel row, so pixel access is never interleaved.
        let output = UnsafeShared::new(output);

        (0..row_count).into_par_iter().for_each(|i| {
            let output = unsafe { output.get() };
            let row_in = &self[i as usize];
            let row_out = &mut output[i as usize];
            <<Self as Index<usize>>::Output as Index<usize>>::Output::convert(row_in, row_out);
        });
    }
}

impl <'a, DP, I> Convert<ImageBuffer<DP>> for I
where
    DP: Pixel,
    DP::T: Copy + Send + Zero,
    I: GenericImageView<'a> + Index<usize> + Sync,
    <I as Index<usize>>::Output: Index<usize>,
    <I as Index<usize>>::Output: AsRef<[<<I as Index<usize>>::Output as Index<usize>>::Output]>,
    <<I as Index<usize>>::Output as Index<usize>>::Output: Pixel + ConvertSlice<DP>,

{
    fn convert(&self, output: &mut ImageBuffer<DP>) {
        if output.width() != self.width() || output.height() != self.height() {
            *output = ImageBuffer::new(self.width(), self.height());
        }

        let row_count = output.height();

        // It is safe to use the shared, lock free wrapper here because each thread
        // accesses a distinct pixel row, so pixel access is never interleaved.
        let output = UnsafeShared::new(output);

        (0..row_count).into_par_iter().for_each(|i| {
            let output = unsafe { output.get() };
            let row_in = &self[i as usize];
            let row_out = &mut output[i as usize];
            <<Self as Index<usize>>::Output as Index<usize>>::Output::convert(row_in, row_out);
        });
    }
}
