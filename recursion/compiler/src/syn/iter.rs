use super::BaseBuilder;
use core::ops::Range;

pub trait IntoIterator<B: BaseBuilder> {
    type Item;

    type IterBuilder<'a>: IterBuilder<Item = Self::Item>
    where
        B: 'a;

    fn into_iter(self, builder: &mut B) -> Self::IterBuilder<'_>;
}

pub trait IterBuilder {
    type Item;

    fn for_each(self, f: impl FnMut(Self::Item, &mut Self));
}

// An iterator for constant size loops.

impl<B: BaseBuilder> IntoIterator<B> for Range<usize> {
    type Item = usize;
    type IterBuilder<'a> = ConstantSizeLoopIterBuilder<'a, B> where B: 'a;

    fn into_iter(self, builder: &mut B) -> Self::IterBuilder<'_> {
        ConstantSizeLoopIterBuilder {
            range: self,
            builder,
        }
    }
}

/// An iterator for constant size loops.
///
/// By default, these loops will be unrolled by the compiler.
pub struct ConstantSizeLoopIterBuilder<'a, B> {
    range: Range<usize>,
    pub(crate) builder: &'a mut B,
}

impl<'a, B: BaseBuilder> BaseBuilder for ConstantSizeLoopIterBuilder<'a, B> {}

impl<'a, B: BaseBuilder> IterBuilder for ConstantSizeLoopIterBuilder<'a, B> {
    type Item = usize;

    fn for_each(mut self, mut f: impl FnMut(usize, &mut Self)) {
        let range = self.range.clone();
        // This is a simple unrolled loop.
        for i in range {
            f(i, &mut self);
        }
    }
}
