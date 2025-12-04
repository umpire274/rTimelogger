use crate::db::pool::DbPool;
use crate::db::queries::{insert_event, load_events_by_date, load_pair_by_index};
use crate::errors::{AppError, AppResult};
use crate::models::event::Event;
use crate::models::event_type::EventType;
use crate::models::location::Location;
use chrono::{NaiveDate, NaiveTime};
use rusqlite::params;

/// High-level business logic for the `add` command.
pub struct AddLogic;

impl AddLogic {
    /// Create or edit events for a given date.
    ///
    /// Combinazioni supportate (edit_mode = false):
    /// - Nessun evento esistente:
    ///   * start -> IN
    ///   * start + end -> IN + OUT
    ///
    /// - Almeno un evento esistente per la data:
    ///   * start -> nuovo IN (nuova coppia aperta)
    ///   * end [+ lunch] -> OUT che chiude l’ultimo IN
    ///   * start + end -> nuova coppia IN/OUT
    ///   * solo lunch -> aggiorna lunch sull’ultimo evento del giorno
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        pool: &mut DbPool,
        date: NaiveDate,
        position: Location,
        start: Option<NaiveTime>,
        lunch: Option<i32>,
        end: Option<NaiveTime>,
        edit_mode: bool,
        edit_pair: Option<usize>,
        _pos: Option<String>,
    ) -> AppResult<()> {
        // ---------------------------------------------
        // CASE 2 — EDIT MODE (full-featured)
        // ---------------------------------------------
        if edit_mode {
            let pair_num = edit_pair.unwrap(); // usize dal CLI

            // carica la N-esima coppia logica IN/OUT del giorno
            let (mut ev_in, mut ev_out) = load_pair_by_index(&pool.conn, &date, pair_num)?;

            // Da qui in poi la logica "C" che avevamo definito:
            // - modifica solo i campi esplicitamente passati
            //   (pos, in, out, lunch)
            // - NON tocca gli altri campi

            // Esempio, in pseudo-codice / patch:

            // 1) POSIZIONE: se hai un'informazione di posizione (dal chiamante),
            //    applicala all'IN e all'OUT esistenti.
            //    Qui assumo che `position` sia il valore finale calcolato dal comando.
            if let Some(ref mut e) = ev_in {
                e.location = position;
            }
            if let Some(ref mut e) = ev_out {
                e.location = position;
            }

            // 2) START: se è stato passato uno start (Option<NaiveTime>),
            //    aggiorna SOLO l'IN; se non c’è IN, decidi se crearlo o no
            if let Some(start_time) = start {
                if let Some(ref mut e) = ev_in {
                    e.time = start_time;
                } else {
                    ev_in = Some(Event::new(
                        0,
                        date,
                        start_time,
                        EventType::In,
                        position,
                        lunch, // opzionale
                        false,
                    ));
                }
            }

            // 3) END: se è stato passato un end, aggiorna SOLO l’OUT
            if let Some(end_time) = end {
                if let Some(ref mut e) = ev_out {
                    e.time = end_time;
                } else {
                    ev_out = Some(Event::new(
                        0,
                        date,
                        end_time,
                        EventType::Out,
                        position,
                        Some(0),
                        false,
                    ));
                }
            }

            // 4) LUNCH: se passato, OUT se esiste, altrimenti IN
            if let Some(lunch_val) = lunch {
                if let Some(ref mut e) = ev_out {
                    e.lunch = Some(lunch_val);
                } else if let Some(ref mut e) = ev_in {
                    e.lunch = Some(lunch_val);
                }
            }

            // 5) Salvataggio: se id == 0 → INSERT, altrimenti UPDATE
            if let Some(ref e) = ev_in {
                if e.id == 0 {
                    insert_event(&pool.conn, e)?;
                } else {
                    crate::db::queries::update_event(&pool.conn, e)?;
                }
            }

            if let Some(ref e) = ev_out {
                if e.id == 0 {
                    insert_event(&pool.conn, e)?;
                } else {
                    crate::db::queries::update_event(&pool.conn, e)?;
                }
            }

            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!("Updated pair {}", pair_num);
            return Ok(());
        }

        let lunch_val = lunch.unwrap_or(0);
        let date_str = date.to_string();

        // Carica eventi già presenti in quella data (ordinati per time ASC)
        let events_today = load_events_by_date(pool, &date)?;
        let has_events = !events_today.is_empty();

        // 1️⃣ CASO SPECIALE: solo lunch su giornata esistente
        if start.is_none() && end.is_none() && lunch.is_some() {
            if !has_events {
                return Err(AppError::InvalidTime(
                    "Non puoi impostare il pranzo su una data senza eventi.".into(),
                ));
            }

            // Aggiorna SOLO il campo lunch_break dell'ultimo evento del giorno
            let updated = pool.conn.execute(
                r#"
                UPDATE events
                SET lunch_break = ?1
                WHERE id = (
                    SELECT id
                    FROM events
                    WHERE date = ?2
                    ORDER BY time DESC
                    LIMIT 1
                )
                "#,
                params![lunch_val, &date_str],
            )?;

            if updated == 0 {
                return Err(AppError::InvalidTime(
                    "Impossibile aggiornare il pranzo: nessun evento da modificare.".into(),
                ));
            }

            println!(
                "Pranzo aggiornato a {} minuti per l'ultimo evento del {}",
                lunch_val, date_str
            );
            return Ok(());
        }

        // 2️⃣ Nessun parametro significativo → errore
        if start.is_none() && end.is_none() {
            return Err(AppError::InvalidTime(
                "Nessuna operazione: specifica almeno --in, --out o --lunch.".into(),
            ));
        }

        // 3️⃣ start solo → sempre nuovo IN (nuova coppia aperta)
        if let Some(start_time) = start
            && end.is_none()
        {
            let ev_in = Event::new(
                0,
                date,
                start_time,
                EventType::In,
                position,
                Some(lunch_val),
                false, // work_gap
            );
            insert_event(&pool.conn, &ev_in)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!(
                "Aggiunto IN: {} {} @ {} (lunch {} min)",
                date_str,
                position.code(),
                start_time,
                lunch_val
            );
            return Ok(());
        }

        // 4️⃣ solo end (con opzionale lunch) → chiude l’ultimo IN
        if start.is_none()
            && let Some(end_time) = end
        {
            // Cerca l'ultimo IN della giornata
            let last_in = events_today
                .iter()
                .rev()
                .find(|ev| ev.kind == EventType::In)
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidTime(
                        "Non posso aggiungere un OUT: nessun IN precedente per questa data.".into(),
                    )
                })?;

            // Controllo semplice: OUT deve essere dopo l'IN
            if end_time <= last_in.time {
                return Err(AppError::InvalidTime(
                    "L'orario di OUT deve essere maggiore dell'ultimo IN.".into(),
                ));
            }

            let ev_out = Event::new(
                0,
                date,
                end_time,
                EventType::Out,
                position,
                Some(lunch_val),
                false,
            );
            insert_event(&pool.conn, &ev_out)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!(
                "Aggiunto OUT: {} {} -> {} (lunch {} min)",
                date_str, last_in.time, end_time, lunch_val
            );
            return Ok(());
        }

        // 5️⃣ start + end → crea una coppia completa IN/OUT
        if let (Some(start_time), Some(end_time)) = (start, end) {
            if end_time <= start_time {
                return Err(AppError::InvalidTime(
                    "L'orario di END deve essere maggiore dell'IN.".into(),
                ));
            }

            let ev_in = Event::new(
                0,
                date,
                start_time,
                EventType::In,
                position,
                Some(lunch_val),
                false,
            );
            let ev_out = Event::new(
                0,
                date,
                end_time,
                EventType::Out,
                position,
                Some(0), // lunch associato al primo evento
                false,
            );

            insert_event(&pool.conn, &ev_in)?;
            insert_event(&pool.conn, &ev_out)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!(
                "Aggiunta coppia per {}: {} -> {} (lunch {} min)",
                date_str, start_time, end_time, lunch_val
            );
            return Ok(());
        }

        // 6️⃣ Fallback
        Err(AppError::InvalidTime(
            "Combinazione di parametri non gestita (bug interno: per favore segnala).".into(),
        ))
    }
}
