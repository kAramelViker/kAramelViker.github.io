use std::default::Default;
use crate::back;
use crate::back::{bounds_month, bounds_none, bounds_today, bounds_week_iso_monday, bounds_year, filter_range, new_data, parse_data, write_data, AppStorage, PurchaseEntry};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui;
use egui::{Color32, Pos2, Stroke, Ui, Widget};
use egui_extras::DatePickerButton;
use egui_timepicker::TimePickerButton;
use elegance::{Card, Theme};
use jiff::civil::{date, Date};
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::vec::Vec;
use uuid::Uuid;

#[derive(Default, Eq, PartialEq)]
pub enum Tabs {
    #[default]
    Main,
    Data,
}
#[derive(Default, Eq, PartialEq)]
pub enum RowAction {
    Save,
    Delete,
    #[default]
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Time {
	#[default]
	Day,
	Week,
	Month,
	Year,
	All,
}
impl Time {
	fn label(self) -> &'static str {
		match self {
			Time::Day => "This Day",
			Time::Week => "This Week",
			Time::Month => "This Month",
			Time::Year => "This Year",
			Time::All => "All Time",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default,Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum SpendingCategory {
	#[default]
	All,
    Groceries,
    Rent,
    Utilities,
    Dining,
    Transport,
    Entertainment,
    Health,
    Shopping,
	Education,
	Other,
}
impl SpendingCategory {
    pub const ALL: [SpendingCategory; 11] = [
	    SpendingCategory::All,
        SpendingCategory::Groceries,
        SpendingCategory::Rent,
        SpendingCategory::Utilities,
        SpendingCategory::Dining,
        SpendingCategory::Transport,
        SpendingCategory::Entertainment,
        SpendingCategory::Health,
        SpendingCategory::Education,
        SpendingCategory::Shopping,
        SpendingCategory::Other,
    ];
	pub const ALLENTERABLE: [SpendingCategory; 10] = [
		SpendingCategory::Groceries,
		SpendingCategory::Rent,
		SpendingCategory::Utilities,
		SpendingCategory::Dining,
		SpendingCategory::Transport,
		SpendingCategory::Entertainment,
		SpendingCategory::Health,
		SpendingCategory::Education,
		SpendingCategory::Shopping,
		SpendingCategory::Other,
	];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Groceries => "Groceries",
            Self::Rent => "Rent",
            Self::Utilities => "Utilities",
            Self::Dining => "Dining",
            Self::Transport => "Transport",
            Self::Entertainment => "Entertainment",
            Self::Health => "Health",
            Self::Education => "Education",
            Self::Shopping => "Shopping",
            Self::Other => "Other",
        }
    }
}

#[derive(Default)]
pub(crate) struct App {
    // dropped_files: Vec<egui::DroppedFile>,
    picked_path: Option<String>,
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    current_tab: Tabs,

    storage: AppStorage,
    overview_time_frame: Time,
    prev_otf: Time,
	overview_map: HashMap<SpendingCategory, f64>,

    editor: PurchaseEntryEditor,
    edit_id: Option<Uuid>,
    new_editor: PurchaseEntryEditor,
    search_editor: SearchEntryEditor,
	last_fingerprint: u64,
	sort_by: SortBy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SortBy {
	#[default]
	DateAsc,
	DateDesc,
	AmountAsc,
	AmountDesc,
	MerchantAsc,
	MerchantDesc,
	CategoryAsc,
	CategoryDesc
}
impl SortBy {
	fn label(self) -> &'static str {
		match self {
			SortBy::DateAsc => "Date ↑",
			SortBy::DateDesc => "Date ↓",
			SortBy::AmountAsc => "Amount ↑",
			SortBy::AmountDesc => "Amount ↓",
			SortBy::MerchantAsc => "Merchant A→Z",
			SortBy::MerchantDesc => "Merchant Z→A",
			SortBy::CategoryAsc => "Category ↑",
			SortBy::CategoryDesc => "Category ↓",
		}
	}
}

#[derive(Default, Clone, Debug)]
pub struct PurchaseEntryEditor {
	pub time: NaiveTime,
	pub date: Date,
	pub amount_text: String,
	pub merchant: String,
	pub category: SpendingCategory,
	pub notes: String,
}

impl PurchaseEntryEditor {
	fn new() -> PurchaseEntryEditor {
		let category = SpendingCategory::Other;
		Self {
			time: NaiveTime::default(),
			date: Date::default(),
			amount_text: String::new(),
			merchant: String::new(),
			category: category.clone(),
			notes: String::new(),
		}
	}

	pub fn apply_to(&self, e: &mut PurchaseEntry) {
		let chrono_date = NaiveDate::from_ymd_opt(
			self.date.year() as i32,
			self.date.month() as u32,
			self.date.day() as u32,
		);

		e.date = chrono_date
			.expect("no date?")
			.and_time(NaiveTime::from(self.time));

		if let Ok(v) = self.amount_text.parse::<f64>() {
			e.amount = v;
		}

		e.merchant = self.merchant.clone();
		e.category = self.category.clone();
		e.notes = self.notes.clone();
	}

	fn from(e: &PurchaseEntry) -> Self {
		Self {
			time: e.date.clone().time(),
			date: date(e.date.year() as i16, e.date.month() as i8, e.date.day() as i8),
			amount_text: e.amount.to_string(),
			merchant: e.merchant.clone(),
			category: e.category.clone(),
			notes: e.notes.clone(),
		}
	}

	fn editor_entry(ui: &mut egui::Ui, editor: &mut PurchaseEntryEditor) {
		ui.horizontal(|ui| {
			ui.add(DatePickerButton::new(&mut editor.date));
			TimePickerButton::ui(TimePickerButton::new(&mut editor.time), ui);

			ui.add_sized(
				[80.0, 20.0],
				egui::TextEdit::singleline(&mut editor.amount_text).hint_text("Amount"),
			);
			ui.add_sized(
				[80.0, 20.0],
				egui::TextEdit::singleline(&mut editor.merchant).hint_text("Merchant"),
			);

			egui::ComboBox::from_label("Category")
				.width(80.0)
				.selected_text(editor.category.as_str())
				.show_ui(ui, |ui| {
					for c in SpendingCategory::ALLENTERABLE.iter().copied() {
						ui.selectable_value(&mut editor.category, c, c.as_str());
					}
				});

			ui.add_sized(
				[80.0, 20.0],
				egui::TextEdit::singleline(&mut editor.notes).hint_text("Notes"),
			);
		});
	}
}

#[derive(Clone, Debug)]
pub struct SearchEntryEditor {
	base: PurchaseEntryEditor,
	pub date_end: Date,
	pub time_end: NaiveTime,

	pub auto_apply: bool,
	last_applied_notes: String,
	last_applied_category: SpendingCategory,
	last_applied_from_date: Date,
	last_applied_from_time: NaiveTime,
	last_applied_to_date: Date,
	last_applied_to_time: NaiveTime,
}

impl Default for SearchEntryEditor {
	fn default() -> SearchEntryEditor {
		Self {
			base: Default::default(),
			date_end: Date::constant(4000,1,1),
			time_end: Default::default(),
			auto_apply: false,
			last_applied_notes: "".to_string(),
			last_applied_category: Default::default(),
			last_applied_from_date: Date::default(),
			last_applied_from_time: NaiveTime::default(),
			last_applied_to_date: Date::MAX,
			last_applied_to_time: NaiveTime::default(),
		}
	}
}

impl SearchEntryEditor {

	fn editor_entry(ui: &mut Ui, editor: &mut SearchEntryEditor) {
		ui.horizontal(|ui| {
			ui.add(DatePickerButton::new(&mut editor.date).id_salt("adfhtdbfnfykmjt"));
			ui.add(DatePickerButton::new(&mut editor.date_end).id_salt("asdffsa"));
			TimePickerButton::ui(TimePickerButton::new(&mut editor.time).id_salt("erthtrh"), ui);
			TimePickerButton::ui(TimePickerButton::new(&mut editor.time_end).id_salt
			("ethdbfthdrgbntghrd"), ui);

			ui.add_sized(
				[80.0, 20.0],
				egui::TextEdit::singleline(&mut editor.amount_text).hint_text("Amount"),
			);
			ui.add_sized(
				[80.0, 20.0],
				egui::TextEdit::singleline(&mut editor.merchant).hint_text("Merchant"),
			);

			egui::ComboBox::from_label("Category")
				.width(80.0)
				.selected_text(editor.category.as_str())
				.show_ui(ui, |ui| {
					for c in SpendingCategory::ALL.iter().copied() {
						ui.selectable_value(&mut editor.category, c, c.as_str());
					}
				});

			ui.add_sized(
				[80.0, 20.0],
				egui::TextEdit::singleline(&mut editor.notes).hint_text("Notes"),
			);
		});
	}

	fn apply_if_changed(&mut self) -> bool {
		if !self.auto_apply { return false; }

		let notes_changed = self.notes != self.last_applied_notes;
		let category_changed = self.category != self.last_applied_category;
		let fdate_changed = self.date != self.last_applied_from_date;
		let tdate_changed = self.date_end != self.last_applied_to_date;
		let ftime_changed = self.time != self.last_applied_from_time;
		let ttime_changed = self.time_end != self.last_applied_to_time;

		let changed = notes_changed || category_changed || fdate_changed || tdate_changed || ftime_changed || ttime_changed;

		if changed {
			// update snapshots
			self.last_applied_notes = self.notes.clone();
			self.last_applied_category = self.category.clone();
			self.last_applied_from_date = self.date.clone();
			self.last_applied_from_time = self.time;
			self.last_applied_to_date = self.date_end.clone();
			self.last_applied_to_time = self.time_end;
		}

		changed
	}

}

impl std::ops::Deref for SearchEntryEditor {
	type Target = PurchaseEntryEditor;
	fn deref(&self) -> &Self::Target { &self.base }
}

impl std::ops::DerefMut for SearchEntryEditor {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.base }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        Theme::paper().install(ui.ctx()); // apply theme

	    // egui::Context::set_debug_on_hover(ui,true);

        egui::Panel::top("tmenu").show(ui, |ui| {
            self.top_menu(ui);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical( |ui| {
                    self.side_panel(ui);
                });
                self.current_tab(ui);
            });

        if ui.input(|i| i.viewport().close_requested()) {
            self.handle_close_request(ui);
        }

        if self.show_confirmation_dialog {
            self.confirm_quit_dialog(ui);
        }
        });
    }
}

fn polar(center: Pos2, r: f64, ang: f64) -> Pos2 {
	Pos2::new(center.x + (r * ang.cos()) as f32, center.y + (r * ang.sin()) as f32)
}

fn draw_thick_arc_band(
	ui: &mut Ui,
	//ughhh drawiong math is jard
	center: Pos2, r_outer: f64, thickness: f64, a0: f64, a1: f64, color: Color32, ) {
	let painter = ui.painter();
	let r_inner = (r_outer - thickness).max(0.0);

	let steps = 80usize; // increase for smoother
	let total = a1 - a0;

	let band_steps = 6usize.max((thickness / 4.0).round() as usize); // tweak
	for s in 0..band_steps {
		let t = s as f64 / (band_steps.saturating_sub(1)).max(1) as f64;
		let r = r_inner + t * (r_outer - r_inner);

		let mut prev = polar(center, r, a0);
		for i in 1..=steps {
			let ang = a0 + total * (i as f64 / steps as f64);
			let p = polar(center, r, ang);
			painter.line_segment([prev, p], Stroke::new(1.5, color));
			prev = p;
		}
	}
}

fn slice_color(cat: SpendingCategory) -> Color32 {
	match cat {
		SpendingCategory::All => Color32::BLACK,
		SpendingCategory::Groceries => Color32::from_rgb(255, 99, 132),
		SpendingCategory::Rent => Color32::from_rgb(54, 162, 235),
		SpendingCategory::Utilities => Color32::from_rgb(255, 206, 86),
		SpendingCategory::Dining => Color32::from_rgb(75, 192, 192),
		SpendingCategory::Transport => Color32::from_rgb(153, 102, 255),
		SpendingCategory::Entertainment => Color32::from_rgb(255, 159, 64),
		SpendingCategory::Health => Color32::from_rgb(0, 200, 255),
		SpendingCategory::Education => Color32::from_rgb(255, 77, 109),
		SpendingCategory::Shopping => Color32::from_rgb(127, 255, 212),
		SpendingCategory::Other => Color32::from_rgb(210, 180, 140),
	}
}

pub fn donut_pie(
	ui: &mut Ui, center: Pos2, radius: f64, thickness: f64,
	values: &HashMap<SpendingCategory, f64>, center_text: String,
) {
	let n = values.keys().count();
	let sum: f64 = values.values().sum();

	let hole_color = ui.ctx().theme().default_visuals().faint_bg_color;
	let hole_r = (radius - thickness).max(0.1);
	ui.painter().circle_filled(center, hole_r as f32, hole_color);

	let mut angle = -std::f64::consts::FRAC_PI_2; // start at top
	let gap = 0.0_f64;

	if n > 0 && sum > 0.0f64 {
		for (cat, v) in values.iter() {
			if *v <= 0.0 { continue; }

			let frac = *v / sum;
			let slice_angle = frac * std::f64::consts::TAU;

			let a0 = angle + gap;
			let a1 = angle + slice_angle - gap;
			angle += slice_angle;

			let color = slice_color(*cat);
			draw_thick_arc_band(ui, center, radius, thickness, a0, a1, color);

			let mid = (a0 + a1) * 0.5;

			let small_font_size = 10.0;
			let large_font_size = 18.0;
			let offset = 30.0;

			let r_label = hole_r + offset;

			let pos = polar(center, r_label, mid);
			let hovered = ui.ctx().pointer_hover_pos().map_or(false, |mouse_pos| {
				(mouse_pos - pos).length() < 10.0 // hover threshold
			});

			if hovered {
				ui.painter().text(
					pos,
					egui::Align2::CENTER_CENTER,
					cat.as_str(),
					egui::FontId::proportional(large_font_size),
					ui.visuals().text_color(),
				);
			} else {
				ui.painter().text(
					pos,
					egui::Align2::CENTER_CENTER,
					cat.as_str(),
					egui::FontId::proportional(small_font_size),
					ui.visuals().gray_out(ui.visuals().text_color()),
				);
			}
		}
	}
	ui.painter().text(
		center,
		egui::Align2::CENTER_CENTER,
		center_text,
		egui::FontId::proportional(18.0),
		ui.visuals().text_color(),
	);

}

impl App {

    pub(crate) fn name() -> &'static str {
        "app"
    }

    fn top_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                self.file_menu(ui);
            });
            ui.menu_button("Tools", |ui| {
                self.tools_menu(ui);
            });
        });
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("New").clicked() {
            self.picked_path = None;
            self.storage.purge();
            self.storage.add(new_data());
            println!("made new thing")
        }

        ui.separator();

        if ui.button("Open new...").clicked() {
            self.open_file_dialog();
        }

        ui.separator();

        if ui.button("Save").clicked() {
			if self.picked_path == None { return; }
            let _ = write_data(self.picked_path.clone(), self.storage.get_all());
        }

        if ui.button("Save as...").clicked() {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                self.picked_path = Some(path.display().to_string());
                let _ = write_data(self.picked_path.clone(), self.storage.get_all());
            }
        }

        ui.separator();

        if ui.button("Quit").clicked() {
            std::process::exit(0);
        }
    }

    fn tools_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Preferences").clicked() {
            unimplemented!()
        }
        if ui.button("Settings").clicked() {
            self.settings_window(ui)
        }
        if ui.button("About").clicked() {
            self.about_window(ui);
        }
    }

    fn open_file_dialog(&mut self) {
        self.storage.purge();
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.picked_path = Some(path.display().to_string());
            for entry in parse_data(self.picked_path.clone()) {
                self.storage.add(entry);
            }
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Tabs").show(ui, |ui| {
            ui.vertical(|ui| {
                self.tab_button(ui, "Overview", Tabs::Main);
                self.tab_button(ui, "Purchases", Tabs::Data);
            });
        });
    }

    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: Tabs) {
        if ui.button(label).clicked() {
            self.current_tab = tab;
        }
    }

    fn current_tab(&mut self, ui: &mut egui::Ui) {
        match self.current_tab {
            Tabs::Main => {
                self.main_tab(ui);
            }
            Tabs::Data => {
	            ui.vertical(|ui| {
		            let h = ui.available_height();
		            ui.allocate_ui_with_layout(
			            egui::Vec2::new(ui.available_width(), h * 0.5),
			            egui::Layout::top_down(egui::Align::Min),
			            |card_ui| {
				            self.data_top_tab(card_ui);
			            },
		            );
		            ui.allocate_ui_with_layout(
			            egui::Vec2::new(ui.available_width(), ui.available_height()),
			            egui::Layout::top_down(egui::Align::Min),
			            |card_ui| {
				            self.data_bot_tab(card_ui);
			            },
		            );

	            });
            },
        }
    }

	fn main_tab(&mut self, ui: &mut egui::Ui) {
		Card::new().heading("Overview").show(ui, |ui| {
			egui::Grid::new("main_stuff")
				.show(ui, |ui| {
					Card::new().heading("Expenses").show(ui, |ui| {
						egui::ComboBox::from_label("Timeframe")
							.selected_text(self.overview_time_frame.label())
							.show_ui(ui, |ui| {
								ui.selectable_value(&mut self.overview_time_frame, Time::Day,   Time::Day.label());
								ui.selectable_value(&mut self.overview_time_frame, Time::Week,  Time::Week.label());
								ui.selectable_value(&mut self.overview_time_frame, Time::Month, Time::Month.label());
								ui.selectable_value(&mut self.overview_time_frame, Time::Year,  Time::Year.label());
								ui.selectable_value(&mut self.overview_time_frame, Time::All, Time::All.label())
							});

						let pie_size = egui::vec2(300.0, 220.0);
						let (rect, _) = ui.allocate_exact_size(pie_size, egui::Sense::hover());

						let center = rect.center();

						let fp = back::fingerprint(&self.storage);
						if fp != self.last_fingerprint || self.prev_otf != self.overview_time_frame {
							self.overview_map.clear();
							self.last_fingerprint = fp;
							println!("FP: last={}, now={}", self.last_fingerprint, fp);
							self.prev_otf = self.overview_time_frame;

							let frame: (NaiveDateTime,NaiveDateTime) = match self.overview_time_frame {
								Time::Day => {bounds_today() }
								Time::Week => {bounds_week_iso_monday()}
								Time::Month => {bounds_month()}
								Time::Year => {bounds_year()}
								Time::All => {bounds_none()}
							};

							let mut filtered_entries: Vec<_> = filter_range(self.storage.get_all(), frame.0, frame.1).into_iter().collect();

							filtered_entries.sort_by_key(|e| e.category.clone());
							for e in filtered_entries {
								*self.overview_map.entry(e.category.clone()).or_insert(0.0) += e.amount;
							}
						}

						let radius:f64 = 90.0;
						let thickness:f64 = 20.0;

						let total = self
							.overview_map
							.values()
							.copied()
							.reduce(|acc, x| acc + x)
							.unwrap_or(0.0);

						donut_pie(ui, center, radius, thickness, &self.overview_map,
						          format!("Total spent \n {:.2}", total));
					});
					ui.end_row();
				});
		});
	}

	fn data_top_tab(&mut self, ui: &mut egui::Ui) {
		Card::new().heading("Purchases").show(ui, |ui| {
			let mut to_save: Vec<Uuid> = Vec::new();
			let mut to_delete: Vec<Uuid> = Vec::new();

			let entries: Vec<PurchaseEntry> = self.storage.get_sorted_filtered(self.sort_by,
			                                                                   self.search_editor.clone())
				.iter().cloned().collect();

			if entries.is_empty() {
				ui.label("Nothing to see here!");
			} else {
				ui.vertical(|ui| {
					egui::ScrollArea::vertical()
						.show(ui, |ui| {
							for index in 0..entries.len() {
								let entry_snapshot = &entries[index];
								let id = entry_snapshot.id;

								ui.push_id(id, |ui| {
									let mut action = RowAction::None;

									if self.edit_id == Some(id) {
										ui.horizontal(|ui| {
											PurchaseEntryEditor::editor_entry(ui, &mut self.editor);

											if ui.button("SAVE").clicked() {
												action = RowAction::Save;
											} else if ui.button("DELETE").clicked() {
												action = RowAction::Delete;
											}
										});
									} else {
										// ensure this call returns nothing or RowAction
										// not InnerResponse pls
										// aaaa
										self.entry_label(ui, id, entry_snapshot);
									}

									if let RowAction::Save = action {
										to_save.push(id);
									}
									if let RowAction::Delete = action {
										to_delete.push(id);
									}
								});
							}
						});
				});

				ui.add_space(8.0);
			}

			egui::Panel::bottom("new_edit").show(ui, |ui| {
				ui.horizontal(|ui| {
					ui.label("New entry");
					PurchaseEntryEditor::editor_entry(ui,&mut self.new_editor);
					if ui.button("ADD").clicked() {
						let mut entry = PurchaseEntry {
							id: Uuid::new_v4(),
							date: NaiveDateTime::default(),
							amount: 0.0,
							merchant: String::new(),
							category: SpendingCategory::Other,
							notes: String::new(),
						};
						self.new_editor.apply_to(&mut entry);
						self.storage.add(entry);
						self.new_editor = PurchaseEntryEditor::new();
					}
				})
			});

			for id in to_delete {
				self.storage.remove(id);
			}

			for id in to_save {
				if let Some(e_mut) = self.storage.get_mut(Some(id)) {
					self.editor.apply_to(e_mut);
					self.edit_id = None;
				}
			}

		});
	}

	fn data_bot_tab(&mut self, ui: &mut egui::Ui) {
		Card::new().heading("Details").show(ui, |ui| {
			if let Some(e) = self.storage.get(self.edit_id) {
				let text = format!(
					"Editing:\n\
			         Date: {}\n\
			         Amount: {:.3}\n\
			         Merchant: {}\n\
			         Category: {}\n\
			         Notes: {}",
					e.date,
					e.amount,
					e.merchant,
					e.category.as_str(),
					e.notes
				);
				ui.label(text);
			} else {
				ui.label("Not editing anything");
			}

			egui::ComboBox::from_label("Sort")
				.selected_text(self.sort_by.label())
				.show_ui(ui, |ui| {
					for opt in [
						SortBy::DateAsc,
						SortBy::DateDesc,
						SortBy::AmountAsc,
						SortBy::AmountDesc,
						SortBy::MerchantAsc,
						SortBy::MerchantDesc,
						SortBy::CategoryAsc,
						SortBy::CategoryDesc
					] {
						ui.selectable_value(&mut self.sort_by, opt, opt.label());
					}
				});

			ui.label("Search by..");
				SearchEntryEditor::editor_entry(ui, &mut self.search_editor);
				if self.search_editor.apply_if_changed() {

				}
		});
	}

    fn entry_label(&mut self, ui: &mut egui::Ui, index: Uuid, e: &PurchaseEntry) {
        let text = format!(
            "{}  {:.3}  {}  {}  {}",
            e.date,
            e.amount,
            e.merchant,
            e.category.as_str(),
            e.notes
        );
        if ui
            .add(egui::Label::new(text).sense(egui::Sense::click()))
            .clicked()
        {
            println!("clicked on editing entry {:?}", index);
            self.edit_id = Some(index);
            self.editor = PurchaseEntryEditor::from(e);
        }
    }

	fn handle_close_request(&mut self, ui: &mut egui::Ui) {
		if self.allowed_to_close {
			// allow close
		} else {
			ui.send_viewport_cmd(egui::ViewportCommand::CancelClose);
			self.show_confirmation_dialog = true;
        }
    }

    fn about_window(&self, ui: &mut egui::Ui) {
        egui::Window::new("About")
            .resizable(false)
            .collapsible(false)
            .show(ui, |ui| {
                ui.label("about");
            });
    }

    fn settings_window(&self, ui: &mut egui::Ui) {
        egui::Window::new("Setting")
            .resizable(false)
            .show(ui, |ui| {
                ui.label("change theme");
                egui::widgets::global_theme_preference_buttons(ui);
                ui.separator();
                ui.label("another setting");
                egui::widgets::global_theme_preference_buttons(ui);
            });
    }

    fn confirm_quit_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Do you want to quit?")
            .pivot(egui::Align2::CENTER_TOP)
            .collapsible(false)
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("No").clicked() {
                        self.show_confirmation_dialog = false;
                        self.allowed_to_close = false;
                    }
                    if ui.button("Yes").clicked() {
                        self.show_confirmation_dialog = false;
                        self.allowed_to_close = true;
                        ui.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
    }
}
