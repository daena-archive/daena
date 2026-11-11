//! Small sequential/parallel dispatch helpers for per-cell loops.

use rayon::prelude::*;

/// Run `body` over every cell of one field. `parallel` only chooses how the
/// body is iterated; both branches run the same body in cell order.
pub(crate) fn for_each_cell_mut<T, F>(field: &mut [T], parallel: bool, body: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync + Send,
{
    if parallel {
        field
            .par_iter_mut()
            .enumerate()
            .for_each(|(cell, slot)| body(cell, slot));
    } else {
        for (cell, slot) in field.iter_mut().enumerate() {
            body(cell, slot);
        }
    }
}
