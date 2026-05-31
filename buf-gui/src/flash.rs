/*
    buf - Tool for flashing USB drives across platforms
    Copyright (C) 2026 Bryson Kelly

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/


use std::sync::mpsc;
use std::thread;

use libbuf::WriteParams;

#[derive(Debug)]
pub enum Progress {
    Update(u64, u64),
    Done { elapsed_secs: f64, bytes: u64 },
    Error(String),
}

pub struct FlashHandle {
    pub rx: mpsc::Receiver<Progress>,
}

pub fn start(params: WriteParams, source_size: u64, target_file: std::fs::File) -> FlashHandle {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let start = std::time::Instant::now();

        let tx_progress = tx.clone();
        let mut on_progress = move |bytes_written: u64| {
            let _ = tx_progress.send(Progress::Update(bytes_written, source_size));
        };

        match libbuf::write(&params, source_size, target_file, Some(&mut on_progress)) {
            Ok(()) => {
                let elapsed = start.elapsed().as_secs_f64();
                let _ = tx.send(Progress::Done { elapsed_secs: elapsed, bytes: source_size });
            }
            Err(e) => {
                let _ = tx.send(Progress::Error(format!("{:#}", e)));
            }
        }
    });

    FlashHandle { rx }
}
