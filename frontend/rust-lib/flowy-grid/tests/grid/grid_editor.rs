#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(unused_imports)]
use crate::grid::block_test::util::GridRowTestBuilder;
use bytes::Bytes;
use flowy_grid::entities::*;
use flowy_grid::services::field::SelectOptionPB;
use flowy_grid::services::field::*;
use flowy_grid::services::grid_editor::{GridPadBuilder, GridRevisionEditor};
use flowy_grid::services::row::{CreateRowRevisionPayload, RowRevisionBuilder};
use flowy_grid::services::setting::GridSettingChangesetBuilder;
use flowy_grid_data_model::revision::*;
use flowy_revision::REVISION_WRITE_INTERVAL_IN_MILLIS;
use flowy_sync::client_grid::GridBuilder;
use flowy_sync::entities::grid::{
    CreateGridFilterParams, DeleteFilterParams, FieldChangesetParams, GridSettingChangesetParams,
};
use flowy_test::helper::ViewTest;
use flowy_test::FlowySDKTest;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use strum::EnumCount;
use strum::IntoEnumIterator;
use tokio::time::sleep;

pub struct GridEditorTest {
    pub sdk: FlowySDKTest,
    pub grid_id: String,
    pub editor: Arc<GridRevisionEditor>,
    pub field_revs: Vec<Arc<FieldRevision>>,
    pub block_meta_revs: Vec<Arc<GridBlockMetaRevision>>,
    pub row_revs: Vec<Arc<RowRevision>>,
    pub field_count: usize,
    pub row_order_by_row_id: HashMap<String, RowPB>,
}

impl GridEditorTest {
    pub async fn new() -> Self {
        let sdk = FlowySDKTest::default();
        let _ = sdk.init_user().await;
        let build_context = make_test_grid();
        let view_data: Bytes = build_context.into();
        let test = ViewTest::new_grid_view(&sdk, view_data.to_vec()).await;
        let editor = sdk.grid_manager.open_grid(&test.view.id).await.unwrap();
        let field_revs = editor.get_field_revs(None).await.unwrap();
        let block_meta_revs = editor.get_block_meta_revs().await.unwrap();
        let row_revs = editor.grid_block_snapshots(None).await.unwrap().pop().unwrap().row_revs;
        assert_eq!(block_meta_revs.len(), 1);

        // It seems like you should add the field in the make_test_grid() function.
        // Because we assert the initialize count of the fields is equal to FieldType::COUNT.
        assert_eq!(field_revs.len(), FieldType::COUNT);

        let grid_id = test.view.id;
        Self {
            sdk,
            grid_id,
            editor,
            field_revs,
            block_meta_revs,
            row_revs,
            field_count: FieldType::COUNT,
            row_order_by_row_id: HashMap::default(),
        }
    }

    pub async fn get_row_revs(&self) -> Vec<Arc<RowRevision>> {
        self.editor
            .grid_block_snapshots(None)
            .await
            .unwrap()
            .pop()
            .unwrap()
            .row_revs
    }

    pub async fn grid_filters(&self) -> Vec<GridFilterConfiguration> {
        let layout_type = GridLayoutType::Table;
        self.editor.get_grid_filter(&layout_type).await.unwrap()
    }

    pub fn get_field_rev(&self, field_type: FieldType) -> &Arc<FieldRevision> {
        self.field_revs
            .iter()
            .filter(|field_rev| {
                let t_field_type: FieldType = field_rev.field_type_rev.into();
                t_field_type == field_type
            })
            .collect::<Vec<_>>()
            .pop()
            .unwrap()
    }

    pub fn block_id(&self) -> &str {
        &self.block_meta_revs.last().unwrap().block_id
    }
}

pub const GOOGLE: &str = "Google";
pub const FACEBOOK: &str = "Facebook";
pub const TWITTER: &str = "Twitter";

pub const COMPLETED: &str = "Completed";
pub const PLANNED: &str = "Planned";
pub const PAUSED: &str = "Paused";

// This grid is assumed to contain all the Fields.
fn make_test_grid() -> BuildGridContext {
    let mut grid_builder = GridBuilder::new();
    // Iterate through the FieldType to create the corresponding Field.
    for field_type in FieldType::iter() {
        let field_type: FieldType = field_type;

        // The
        match field_type {
            FieldType::RichText => {
                let text_field = FieldBuilder::new(RichTextTypeOptionBuilder::default())
                    .name("Name")
                    .visibility(true)
                    .build();
                grid_builder.add_field(text_field);
            }
            FieldType::Number => {
                // Number
                let number = NumberTypeOptionBuilder::default().set_format(NumberFormat::USD);
                let number_field = FieldBuilder::new(number).name("Price").visibility(true).build();
                grid_builder.add_field(number_field);
            }
            FieldType::DateTime => {
                // Date
                let date = DateTypeOptionBuilder::default()
                    .date_format(DateFormat::US)
                    .time_format(TimeFormat::TwentyFourHour);
                let date_field = FieldBuilder::new(date).name("Time").visibility(true).build();
                grid_builder.add_field(date_field);
            }
            FieldType::SingleSelect => {
                // Single Select
                let single_select = SingleSelectTypeOptionBuilder::default()
                    .add_option(SelectOptionPB::new(COMPLETED))
                    .add_option(SelectOptionPB::new(PLANNED))
                    .add_option(SelectOptionPB::new(PAUSED));
                let single_select_field = FieldBuilder::new(single_select).name("Status").visibility(true).build();
                grid_builder.add_field(single_select_field);
            }
            FieldType::MultiSelect => {
                // MultiSelect
                let multi_select = MultiSelectTypeOptionBuilder::default()
                    .option(SelectOptionPB::new(GOOGLE))
                    .option(SelectOptionPB::new(FACEBOOK))
                    .option(SelectOptionPB::new(TWITTER));
                let multi_select_field = FieldBuilder::new(multi_select)
                    .name("Platform")
                    .visibility(true)
                    .build();
                grid_builder.add_field(multi_select_field);
            }
            FieldType::Checkbox => {
                // Checkbox
                let checkbox = CheckboxTypeOptionBuilder::default();
                let checkbox_field = FieldBuilder::new(checkbox).name("is urgent").visibility(true).build();
                grid_builder.add_field(checkbox_field);
            }
            FieldType::URL => {
                // URL
                let url = URLTypeOptionBuilder::default();
                let url_field = FieldBuilder::new(url).name("link").visibility(true).build();
                grid_builder.add_field(url_field);
            }
        }
    }

    // We have many assumptions base on the number of the rows, so do not change the number of the loop.
    for i in 0..5 {
        let block_id = grid_builder.block_id().to_owned();
        let field_revs = grid_builder.field_revs();
        let mut row_builder = GridRowTestBuilder::new(&block_id, field_revs);
        match i {
            0 => {
                for field_type in FieldType::iter() {
                    match field_type {
                        FieldType::RichText => row_builder.insert_text_cell("A"),
                        FieldType::Number => row_builder.insert_number_cell("1"),
                        FieldType::DateTime => row_builder.insert_date_cell("1647251762"),
                        FieldType::SingleSelect => {
                            row_builder.insert_single_select_cell(|mut options| options.remove(0))
                        }
                        FieldType::Checkbox => row_builder.insert_checkbox_cell("true"),
                        _ => "".to_owned(),
                    };
                }
            }
            1 => {
                for field_type in FieldType::iter() {
                    match field_type {
                        FieldType::RichText => row_builder.insert_text_cell("B"),
                        FieldType::Number => row_builder.insert_number_cell("2"),
                        FieldType::DateTime => row_builder.insert_date_cell("1647251762"),
                        FieldType::SingleSelect => {
                            row_builder.insert_single_select_cell(|mut options| options.remove(0))
                        }
                        FieldType::Checkbox => row_builder.insert_checkbox_cell("true"),
                        _ => "".to_owned(),
                    };
                }
            }
            2 => {
                for field_type in FieldType::iter() {
                    match field_type {
                        FieldType::RichText => row_builder.insert_text_cell("C"),
                        FieldType::Number => row_builder.insert_number_cell("3"),
                        FieldType::DateTime => row_builder.insert_date_cell("1647251762"),
                        FieldType::SingleSelect => {
                            row_builder.insert_single_select_cell(|mut options| options.remove(1))
                        }
                        FieldType::Checkbox => row_builder.insert_checkbox_cell("false"),
                        _ => "".to_owned(),
                    };
                }
            }
            3 => {
                for field_type in FieldType::iter() {
                    match field_type {
                        FieldType::RichText => row_builder.insert_text_cell("D"),
                        FieldType::Number => row_builder.insert_number_cell("4"),
                        FieldType::DateTime => row_builder.insert_date_cell("1647251762"),
                        FieldType::SingleSelect => {
                            row_builder.insert_single_select_cell(|mut options| options.remove(1))
                        }
                        FieldType::Checkbox => row_builder.insert_checkbox_cell("false"),
                        _ => "".to_owned(),
                    };
                }
            }
            4 => {
                for field_type in FieldType::iter() {
                    match field_type {
                        FieldType::RichText => row_builder.insert_text_cell("E"),
                        FieldType::Number => row_builder.insert_number_cell("5"),
                        FieldType::DateTime => row_builder.insert_date_cell("1647251762"),
                        FieldType::SingleSelect => {
                            row_builder.insert_single_select_cell(|mut options| options.remove(2))
                        }

                        FieldType::Checkbox => row_builder.insert_checkbox_cell("false"),
                        _ => "".to_owned(),
                    };
                }
            }
            _ => {}
        }

        let row_rev = row_builder.build();
        grid_builder.add_row(row_rev);
    }
    grid_builder.build()
}
