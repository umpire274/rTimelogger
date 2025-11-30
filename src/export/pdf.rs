use pdf_writer::{Content, Name, Pdf, Rect, Ref};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct PdfManager {
    pdf: Pdf,
    contents_id: Ref,
    page_w: f32,
    page_h: f32,
    margin: f32,
    row_h: f32,
}

impl Default for PdfManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfManager {
    pub fn new() -> Self {
        let mut pdf = Pdf::new();

        let catalog_id = Ref::new(1);
        let pages_id = Ref::new(2);
        let page_id = Ref::new(3);
        let font_id = Ref::new(4);
        let contents_id = Ref::new(5);

        // Catalogo / albero pagine
        pdf.catalog(catalog_id).pages(pages_id);
        pdf.pages(pages_id).kids([page_id]).count(1);

        // A4
        let (page_w, page_h) = (595.0, 842.0);

        // Pagina
        {
            let mut page = pdf.page(page_id);
            page.parent(pages_id)
                .media_box(Rect::new(0.0, 0.0, page_w, page_h))
                .contents(contents_id);
            page.resources().fonts().pair(Name(b"F1"), font_id);
        }

        // Font
        pdf.type1_font(font_id).base_font(Name(b"Helvetica"));

        Self {
            pdf,
            contents_id,
            page_w,
            page_h,
            margin: 50.0,
            row_h: 18.0,
        }
    }

    /// Scrive una riga di testo in tabella alle coordinate (x, y)
    fn write_row(
        &self,
        content: &mut Content,
        y: f32,
        col_w: f32,
        table_x: f32,
        row: &[String],
        bold: bool,
    ) {
        content.begin_text();
        content.set_font(Name(b"F1"), if bold { 11.0 } else { 10.0 });

        for (i, text) in row.iter().enumerate() {
            let cell_x = table_x + (i as f32) * col_w + 4.0;
            // baseline a 5 pt dal bordo inferiore della cella
            content.set_text_matrix([1.0, 0.0, 0.0, 1.0, cell_x, y + 5.0]);
            content.show(pdf_writer::Str(text.as_bytes()));
        }

        content.end_text();
    }

    /// Scrive una tabella semplice con header e righe di dati
    pub fn write_table(&mut self, headers: &[&str], rows: &[Vec<String>]) {
        let table_x = self.margin;
        let mut y = self.page_h - self.margin; // partiamo dall’alto
        let cols = headers.len().max(1) as f32;
        let table_w = self.page_w - 2.0 * self.margin;
        let col_w = table_w / cols;

        let mut content = Content::new();

        // --- Header background ---
        y -= self.row_h;
        content.save_state();
        content.set_fill_rgb(0.90, 0.90, 0.90);
        content.rect(table_x, y, table_w, self.row_h).fill_nonzero();
        content.restore_state();

        // --- Header text (bold “simulato” aumentando la size) ---
        content.begin_text();
        content.set_font(Name(b"F1"), 11.0);
        let header_row: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
        self.write_row(&mut content, y, col_w, table_x, &header_row, true);
        content.end_text();

        // --- Data rows ---
        for (r, row) in rows.iter().enumerate() {
            // salto pagina minimale (solo una pagina in questo esempio)
            if y - self.row_h < self.margin {
                break;
            }

            y -= self.row_h;

            // zebra stripe (righe pari grigio chiaro)
            if r % 2 == 0 {
                content.save_state();
                content.set_fill_rgb(0.96, 0.96, 0.96);
                content.rect(table_x, y, table_w, self.row_h).fill_nonzero();
                content.restore_state();
            }

            content.begin_text();
            content.set_font(Name(b"F1"), 10.0);
            self.write_row(&mut content, y, col_w, table_x, row, false);
            content.end_text();
        }

        // Scrivi lo stream di contenuto
        self.pdf.stream(self.contents_id, &content.finish());
    }

    pub fn save(self, path: &Path) -> std::io::Result<()> {
        let bytes = self.pdf.finish();
        let mut f = File::create(path)?;
        f.write_all(&bytes)?;
        Ok(())
    }
}
