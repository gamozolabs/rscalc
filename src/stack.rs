use crate::edit::NumberEditor;
use crate::font::{SANS_16, SANS_20, SANS_24};
use crate::num_bigint::ToBigInt;
use crate::number::{Number, NumberFormat};
use crate::screen::{Color, Rect, Screen};
use alloc::string::ToString;
use alloc::vec::Vec;

pub struct Stack {
	entries: Vec<Number>,
	editor: Option<NumberEditor>,
	push_new_entry: bool,
}

impl Stack {
	pub fn new() -> Self {
		let zero: Number = 0.into();
		let mut entries = Vec::new();
		entries.push(zero);
		Stack {
			entries,
			editor: None,
			push_new_entry: false,
		}
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn push(&mut self, num: Number) {
		self.entries.push(num);
		self.push_new_entry = true;
		self.editor = None;
	}

	pub fn entry(&self, idx: usize) -> &Number {
		&self.entries[(self.entries.len() - 1) - idx]
	}

	pub fn entry_mut(&mut self, idx: usize) -> &mut Number {
		let len = self.entries.len();
		&mut self.entries[(len - 1) - idx]
	}

	pub fn top(&self) -> &Number {
		self.entry(0)
	}

	pub fn top_mut(&mut self) -> &mut Number {
		self.entry_mut(0)
	}

	pub fn set_top(&mut self, num: Number) {
		*self.top_mut() = num;
		self.push_new_entry = true;
		self.editor = None;
	}

	pub fn replace_entries(&mut self, count: usize, num: Number) {
		for _ in 1..count {
			self.pop();
		}
		self.set_top(num);
		self.push_new_entry = true;
		self.editor = None;
	}

	pub fn pop(&mut self) -> Number {
		let result = self.entries.pop().unwrap();
		if self.entries.len() == 0 {
			self.entries.push(0.into());
		}
		self.push_new_entry = true;
		self.editor = None;
		result
	}

	pub fn enter(&mut self) {
		self.push(self.top().clone());
		self.push_new_entry = false;
	}

	pub fn push_char(&mut self, ch: char) {
		if self.editor.is_none() {
			if self.push_new_entry {
				self.push(0.into());
			} else {
				self.set_top(0.into());
			}
			self.editor = Some(NumberEditor::new_decimal());
			self.push_new_entry = false;
		}
		if let Some(cur_editor) = &mut self.editor {
			if cur_editor.push_char(ch) {
				let value = cur_editor.number();
				*self.top_mut() = value;
			}
		}
	}

	pub fn exponent(&mut self) {
		if let Some(cur_editor) = &mut self.editor {
			cur_editor.exponent();
			let value = cur_editor.number();
			*self.top_mut() = value;
		}
	}

	pub fn backspace(&mut self) {
		if let Some(cur_editor) = &mut self.editor {
			if cur_editor.backspace() {
				let value = cur_editor.number();
				*self.top_mut() = value;
			} else {
				self.set_top(0.into());
				self.push_new_entry = false;
			}
		} else {
			let mut new_entry = self.entries.len() > 1;
			self.pop();
			if let Number::Integer(int) = self.top() {
				if int == &0.to_bigint().unwrap() {
					new_entry = false;
				}
			}
			self.push_new_entry = new_entry;
		}
	}

	pub fn neg(&mut self) {
		if let Some(cur_editor) = &mut self.editor {
			cur_editor.neg();
			let value = cur_editor.number();
			*self.top_mut() = value;
		} else {
			let value = -self.top();
			self.set_top(value);
		}
	}

	pub fn render<ScreenT: Screen>(&self, screen: &mut ScreenT, format: &NumberFormat, area: Rect) {
		let mut bottom = area.y + area.h;

		for idx in 0..self.len() {
			if idx > 0 {
				// Render stack entry separator
				screen.horizontal_pattern(
					area.x,
					area.w,
					bottom,
					0b100100100100100100100100,
					24,
					Color::StackSeparator,
				);
			}

			// Construct and measure stack entry label
			let label = match idx {
				0 => "x".to_string(),
				1 => "y".to_string(),
				2 => "z".to_string(),
				_ => Number::Integer((idx + 1).into()).to_str(),
			};
			let label = label + ": ";
			let label_width = 4 + SANS_16.width(&label);

			// Render stack entry
			let entry = self.entry(idx);
			let height = render_entry(
				screen,
				format,
				if idx == 0 { &self.editor } else { &None },
				entry,
				area.x + label_width,
				area.w - label_width - 4,
				bottom,
			);

			// Draw the label
			SANS_16.draw(
				screen,
				4,
				(bottom - height) + (height - SANS_16.height) / 2,
				&label,
				Color::StackLabelText,
			);

			bottom -= height;
		}
	}
}

fn render_entry<ScreenT: Screen>(
	screen: &mut ScreenT,
	format: &NumberFormat,
	editor: &Option<NumberEditor>,
	value: &Number,
	x: i32,
	w: i32,
	bottom: i32,
) -> i32 {
	// Get string for number. If there is an editor, use editor state instead.
	let string = match editor {
		Some(editor) => editor.to_str(format),
		None => format.format_number(value),
	};

	// Check for alternate representation strings
	let mut alt_string = match value {
		Number::Integer(int) => {
			// Integer, if number is ten or greater check for the
			// hexadecimal alternate form
			if format.show_alt_hex
				&& (int <= &-10.to_bigint().unwrap() || int >= &10.to_bigint().unwrap())
			{
				if format.integer_radix == 10 {
					Some(format.hex_format().format_number(value))
				} else if format.integer_radix == 16 {
					Some(format.decimal_format().format_number(value))
				} else {
					None
				}
			} else {
				None
			}
		}
		Number::Rational(_, _) => {
			// Rational, show floating point as alternate form if enabled
			if format.show_alt_float {
				Some(format.decimal_format().format_decimal(&value.to_decimal()))
			} else {
				None
			}
		}
		Number::Decimal(_) => None,
	};

	let mut top = bottom;
	let mut y = top;

	let mut rational = false;
	if let Number::Rational(num, denom) = value {
		// Rational number, display as an integer and fraction
		top -= SANS_20.height * 2;
		if alt_string.is_some() {
			top -= SANS_16.height;
		}

		// Break rational into an integer part and fractional part
		let int = num / denom.to_bigint().unwrap();
		let mut num = if &int < &0.to_bigint().unwrap() {
			-num - -&int * &denom.to_bigint().unwrap()
		} else {
			num - &int * &denom.to_bigint().unwrap()
		};

		// Get strings for the parts of the rational
		let int_str = if int == 0.to_bigint().unwrap() {
			if &num < &0.to_bigint().unwrap() {
				num = -num;
				"-".to_string()
			} else {
				"".to_string()
			}
		} else {
			format.format_bigint(&int) + " "
		};
		let num_str = format.format_bigint(&num);
		let denom_str = format.format_bigint(&denom.to_bigint().unwrap());

		// Find sizes for the parts of the rational
		let int_width = SANS_24.width(&int_str);
		let num_width = SANS_20.width(&num_str);
		let denom_width = SANS_20.width(&denom_str);
		let fraction_width = core::cmp::max(num_width, denom_width);

		// Check fractional representation width
		let total_width = int_width + fraction_width;
		if total_width <= w {
			// Fractional representation fits, draw integer part
			y = top;
			SANS_24.draw(
				screen,
				x + w - (int_width + fraction_width + 4),
				y + SANS_20.height - (SANS_24.height / 2),
				&int_str,
				Color::ContentText,
			);

			// Draw numerator
			SANS_20.draw(
				screen,
				x + w - (4 + fraction_width / 2) - (num_width / 2),
				y,
				&num_str,
				Color::ContentText,
			);

			// Draw line between numerator and denominator
			screen.fill(
				Rect {
					x: x + w - (fraction_width + 4),
					y: y + SANS_20.height,
					w: fraction_width,
					h: 1,
				},
				Color::ContentText,
			);

			// Draw denominator
			SANS_20.draw(
				screen,
				x + w - (4 + fraction_width / 2) - (denom_width / 2),
				y + SANS_20.height,
				&denom_str,
				Color::ContentText,
			);

			y += SANS_20.height * 2;
			rational = true;
		} else {
			// Fractional representation is too wide, represent as float
			top = bottom;
			alt_string = None;
		}
	}

	if !rational {
		// Integer or decimal float, render string formatted earlier
		top -= SANS_24.height;
		if alt_string.is_some() {
			top -= SANS_16.height;
		}

		// Render string
		y = top;
		let width = SANS_24.width(&string) + 4;
		SANS_24.draw(screen, x + w - width, y, &string, Color::ContentText);

		if editor.is_some() {
			// If there is an editor, render cursor
			screen.fill(
				Rect {
					x: x + w - 3,
					y,
					w: 3,
					h: SANS_24.height,
				},
				Color::ContentText,
			);
		}

		y += SANS_24.height;
	}

	// Render alternate string if there was one
	if let Some(alt_string) = alt_string {
		let width = SANS_16.width(&alt_string) + 4;
		SANS_16.draw(screen, x + w - width, y, &alt_string, Color::ContentText);
	}

	bottom - top
}
