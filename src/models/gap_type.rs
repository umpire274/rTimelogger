#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapType {
    Normal, // non conteggiato come lavoro
    Work,   // conteggiato come lavoro (work_gap = true)
}
