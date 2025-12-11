use pdf_writer::{Content, Name, Pdf, Rect, Ref};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct PdfManager {
    pdf: Pdf,
    catalog_id: Ref,
    pages_id: Ref,
    page_refs: Vec<Ref>,
    current_content_id: Option<Ref>,

    page_w: f32,
    page_h: f32,
    margin: f32,
    row_h: f32,

    next_id: i32,
    font_id: Ref,

    font_size: f32,
    header_font_size: f32,
    title_font_size: f32,
}

impl Default for PdfManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfManager {
    pub fn new() -> Self {
        let mut pdf = Pdf::new();

        // ID gestiti a mano
        let catalog_id = Ref::new(1);
        let pages_id = Ref::new(2);
        let font_id = Ref::new(3);
        let next_id = 4;

        // Font globale
        pdf.type1_font(font_id).base_font(Name(b"Helvetica"));

        Self {
            pdf,
            catalog_id,
            pages_id,
            page_refs: Vec::new(),
            current_content_id: None,

            page_w: 595.0,
            page_h: 842.0,
            margin: 50.0,
            row_h: 20.0,

            next_id,
            font_id,

            font_size: 10.0,
            header_font_size: 11.0,
            title_font_size: 14.0,
        }
    }

    /// Genera un nuovo Ref univoco
    fn fresh_ref(&mut self) -> Ref {
        let id = self.next_id;
        self.next_id += 1;
        Ref::new(id)
    }

    /// Crea una nuova pagina e relativo oggetto di contenuto
    fn new_page(&mut self) -> Content {
        let page_id = self.fresh_ref();
        let content_id = self.fresh_ref();

        self.page_refs.push(page_id);

        let mut page = self.pdf.page(page_id);
        page.parent(self.pages_id)
            .media_box(Rect::new(0.0, 0.0, self.page_w, self.page_h))
            .contents(content_id);

        page.resources().fonts().pair(Name(b"F1"), self.font_id);

        self.current_content_id = Some(content_id);

        Content::new()
    }

    /// Scrive lo stream della pagina corrente
    fn finalize_page(&mut self, content: Content) {
        if let Some(id) = self.current_content_id {
            self.pdf.stream(id, &content.finish());
        }
    }

    /// Imposta il nodo `Pages` con count e kids
    fn build_pages_tree(&mut self) {
        let mut pages = self.pdf.pages(self.pages_id);
        pages.count(self.page_refs.len() as i32);
        pages.kids(self.page_refs.clone());
    }

    fn draw_text(&self, content: &mut Content, x: f32, y: f32, size: f32, text: &str) {
        content.begin_text();
        content.set_font(Name(b"F1"), size);
        content.set_text_matrix([1.0, 0.0, 0.0, 1.0, x, y]);
        content.show(pdf_writer::Str(text.as_bytes()));
        content.end_text();
    }

    fn draw_cell_borders(&self, content: &mut Content, x: f32, y: f32, w: f32, h: f32) {
        content.save_state();
        content.set_stroke_rgb(0.65, 0.65, 0.65);
        content.rect(x, y, w, h);
        content.stroke();
        content.restore_state();
    }

    fn draw_row(
        &self,
        content: &mut Content,
        y: f32,
        col_widths: &[f32],
        x_start: f32,
        row: &[String],
        font_size: f32,
    ) {
        let mut x = x_start;

        for (i, text) in row.iter().enumerate() {
            let w = col_widths[i];
            self.draw_text(content, x + 4.0, y + 5.0, font_size, text);
            self.draw_cell_borders(content, x, y, w, self.row_h);
            x += w;
        }
    }

    /// Calcola larghezza colonne in base a header + contenuto e le adatta alla pagina
    fn compute_col_widths(&self, headers: &[&str], rows: &[Vec<String>]) -> Vec<f32> {
        let mut widths: Vec<f32> = headers.iter().map(|h| h.len() as f32 * 6.5).collect();

        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                let w = (cell.len() as f32 * 6.2).max(widths[i]);
                widths[i] = w;
            }
        }

        let total: f32 = widths.iter().sum();
        let max = self.page_w - 2.0 * self.margin;

        if total > max {
            let scale = max / total;
            for w in &mut widths {
                *w *= scale;
            }
        }

        widths
    }

    fn draw_page_header_footer(&self, content: &mut Content, title: &str, page: usize) {
        // Titolo
        self.draw_text(
            content,
            self.margin,
            self.page_h - self.margin + 15.0,
            self.title_font_size,
            title,
        );

        // Numero pagina
        let pg = format!("Page {}", page);
        self.draw_text(
            content,
            self.page_w - self.margin - 60.0,
            self.margin - 35.0,
            self.font_size,
            &pg,
        );
    }

    /// Tabella multipagina con titolo
    pub fn write_table(&mut self, title: &str, headers: &[&str], rows: &[Vec<String>]) {
        // Se non ci sono righe, evita PDF vuoto: una pagina con solo header
        if rows.is_empty() {
            let col_widths = self.compute_col_widths(headers, &[]);
            let header_row: Vec<String> = headers.iter().map(|s| s.to_string()).collect();

            let mut content = self.new_page();
            self.draw_page_header_footer(&mut content, title, 1);

            let y = self.page_h - self.margin - 30.0;

            content.save_state();
            content.set_fill_rgb(0.85, 0.87, 0.90);
            content.rect(self.margin, y, col_widths.iter().sum(), self.row_h);
            content.fill_nonzero();
            content.restore_state();

            self.draw_row(
                &mut content,
                y,
                &col_widths,
                self.margin,
                &header_row,
                self.header_font_size,
            );

            self.finalize_page(content);
            return;
        }

        let col_widths = self.compute_col_widths(headers, rows);
        let header_row: Vec<String> = headers.iter().map(|s| s.to_string()).collect();

        let mut remaining: &[Vec<String>] = rows;
        let mut page_idx = 1;

        while !remaining.is_empty() {
            let mut content = self.new_page();
            self.draw_page_header_footer(&mut content, title, page_idx);

            let mut y = self.page_h - self.margin - 30.0;

            // header tabella
            content.save_state();
            content.set_fill_rgb(0.85, 0.87, 0.90);
            content.rect(self.margin, y, col_widths.iter().sum(), self.row_h);
            content.fill_nonzero();
            content.restore_state();

            self.draw_row(
                &mut content,
                y,
                &col_widths,
                self.margin,
                &header_row,
                self.header_font_size,
            );

            y -= self.row_h;

            let mut consumed = 0;

            for (i, row) in remaining.iter().enumerate() {
                if y - self.row_h < self.margin {
                    break;
                }

                // zebra stripe
                if i % 2 == 0 {
                    content.save_state();
                    content.set_fill_rgb(0.96, 0.96, 0.96);
                    content.rect(self.margin, y, col_widths.iter().sum(), self.row_h);
                    content.fill_nonzero();
                    content.restore_state();
                }

                self.draw_row(
                    &mut content,
                    y,
                    &col_widths,
                    self.margin,
                    row,
                    self.font_size,
                );

                y -= self.row_h;
                consumed += 1;
            }

            self.finalize_page(content);
            remaining = &remaining[consumed..];
            page_idx += 1;
        }
    }

    pub fn save(mut self, path: &Path) -> std::io::Result<()> {
        // Costruisci Catalog + Pages una sola volta, qui
        self.pdf.catalog(self.catalog_id).pages(self.pages_id);
        self.build_pages_tree();

        let bytes = self.pdf.finish();
        let mut f = File::create(path)?;
        f.write_all(&bytes)?;
        Ok(())
    }
}
