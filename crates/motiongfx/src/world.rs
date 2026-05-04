use crate::subject::SubjectId;

/// Provides read and write access to a source type `S` by subject id `I`.
pub trait SubjectSource<I: SubjectId, S: 'static> {
    fn get_source(&self, id: I) -> Option<&S>;

    fn apply_source<R>(
        &mut self,
        id: I,
        f: impl FnOnce(&mut S) -> R,
    ) -> Option<R>;
}
