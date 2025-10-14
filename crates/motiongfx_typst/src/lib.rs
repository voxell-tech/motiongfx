use std::path::PathBuf;

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_vello_renderer::{
    prelude::*,
    vello_svg::usvg::{self, TreeParsing},
};
use ecow::EcoVec;
use motiongfx_vello::svg;
use typst::{diag::SourceDiagnostic, eval::Tracer, layout::Abs, model::Document};

use crate::world::TypstWorld;

pub mod world;

mod download;
mod fonts;
mod package;

#[derive(Default)]
pub struct TypstCompilerPlugin {
    font_paths: Vec<PathBuf>,
}

impl TypstCompilerPlugin {
    pub fn new(font_paths: Vec<PathBuf>) -> Self {
        Self { font_paths }
    }
}

impl Plugin for TypstCompilerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TypstCompiler::new(&self.font_paths));
    }
}

/// A compiler for compiling Typst content.
///
/// This compiler can be accessed from the resource:
/// ```
/// pub fn compile_system(
///     mut commands: Commands,
///     mut typst_compiler: ResMut<TypstCompiler>,
///     mut fragment_assets: ResMut<Assets<VelloFragment>>,
/// ) {
///     let content: String = String::from(
///         r###"
///         = Introduction
///         + First element.
///         + Second element.
///         "###,
///     );
///
///     match typst_compiler.compile_flatten(&mut commands, &mut fragment_assets, content) {
///         Ok(tree) => {
///             println!("{:#?}", tree.size);
///         }
///         Err(_) => todo!(),
///     }
/// }
/// ```
#[derive(Resource)]
pub struct TypstCompiler {
    world: TypstWorld,
    tracer: Tracer,
}

impl TypstCompiler {
    pub fn new(font_paths: &[PathBuf]) -> Self {
        Self {
            world: TypstWorld::new(PathBuf::from(".\\assets"), font_paths).unwrap(),
            tracer: Tracer::new(),
        }
    }

    pub fn compile(
        &mut self,
        commands: &mut Commands,
        fragment_assets: &mut ResMut<Assets<VelloFragment>>,
        text: String,
    ) -> Result<Entity, EcoVec<SourceDiagnostic>> {
        let tree: usvg::Tree = self.compile_text(text)?;

        Ok(svg::spawn_tree(commands, fragment_assets, &tree))
    }

    /// [`SvgTreeBundle`]: svg::SvgTreeBundle
    /// Compiles the Typst content into Svg and flatten the Svg hierarchy into a [`SvgTreeBundle`].
    ///
    /// If an error occur during Typst compilation, an error message will be returned instead.
    pub fn compile_flatten(
        &mut self,
        commands: &mut Commands,
        fragment_assets: &mut ResMut<Assets<VelloFragment>>,
        text: String,
    ) -> Result<svg::SvgTreeBundle, EcoVec<SourceDiagnostic>> {
        let tree: usvg::Tree = self.compile_text(text)?;

        Ok(svg::spawn_tree_flatten(commands, fragment_assets, &tree))
    }

    fn compile_text(&mut self, text: String) -> Result<usvg::Tree, EcoVec<SourceDiagnostic>> {
        self.world.set_source(text);
        let document: Document = typst::compile(&self.world, &mut self.tracer)?;

        let svg: String = typst_svg::svg_merged(&document.pages, Abs::zero());

        // Svg string should not have any issue if compilation succeeded
        Ok(usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap())
    }
}
