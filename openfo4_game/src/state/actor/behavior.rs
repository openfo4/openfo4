use std::collections::HashMap;

use openfo4_animation::AnimationGraph;
use openfo4_animation::builder::{self, Builder, Hint};
use openfo4_asset::{AssetLoader, Handle};
use openfo4_core::path::{DataPath, DataPathBuf};
use openfo4_hkx::HkxFile;
use openfo4_tasks::TaskPool;

use crate::assets::Error;
use crate::assets::animation::{AnimationSet, FileAnimationClip};
use crate::assets::data_file::DataFile;
use crate::state::animation_test::ActorCharacter;

#[derive(Clone, Debug)]
pub enum LoadBehaviorError {
    Load(Vec<(DataPathBuf, Error)>),
    Build(builder::Error),
    InvalidbehaviorFile(Error),
}

/// Descriptor for [`ActorBehaviorData::load`].
#[derive(Clone, Debug)]
pub struct ActorBehaviorDescriptor<'a, T> {
    /// The path to the root behavior file.
    pub behavior_path: DataPathBuf,
    /// The path to the subgraph behavior file.
    pub subgraph_path: DataPathBuf,
    pub root_path: &'a DataPath,
    pub character: &'a ActorCharacter,
    /// The list of directories where animations will used from.
    ///
    /// If two directories contain an animation file, the earlier element in the list is preferred.
    pub search_paths: &'a [T],
}

impl<'a, T> ActorBehaviorDescriptor<'a, T> {
    fn fix(&mut self) {
        if !self.behavior_path.starts_with(DataPath::from_str("meshes")) {
            self.behavior_path.push_front("meshes");
        }

        if !self.subgraph_path.starts_with(DataPath::from_str("meshes")) {
            self.subgraph_path.push_front("meshes");
        }
    }
}

pub struct ActorBehaviorData {
    pub graph: AnimationGraph,
    pub clips: AnimationSet,
    pub behavior_files: Vec<DataPathBuf>,
}

impl ActorBehaviorData {
    pub async fn load<'a, T>(
        loader: &AssetLoader,
        pool: &TaskPool,
        mut descriptor: ActorBehaviorDescriptor<'_, T>,
    ) -> Result<Self, LoadBehaviorError>
    where
        T: AsRef<DataPath>,
    {
        descriptor.fix();

        let subgraph_root = extract_behavior_root_path(&descriptor.subgraph_path);

        let root = loader
            .load::<DataFile<HkxFile>, _>(&descriptor.behavior_path)
            .await
            .map_err(LoadBehaviorError::InvalidbehaviorFile)?;

        let mut builder = Builder::new((**root).clone(), descriptor.character.props.clone())
            .map_err(LoadBehaviorError::Build)?;

        let mut behavior_files = Vec::new();

        let graph = loop {
            match builder.build().map_err(LoadBehaviorError::Build)? {
                Ok(graph) => break graph,
                Err(Hint::NeedFile(path)) => {
                    let mut file_loaded = false;
                    let mut errors = Vec::new();

                    for p in [
                        descriptor.root_path.join(&*path),
                        // Loading from the subgraph directory can also happen.
                        subgraph_root.join(&*path),
                        // The game seems to also load from this directory for all races.
                        // This doesn't seem to be referenced anywhere, so it might be hardcoded?
                        DataPath::from_str("meshes/actors/shared").join(&*path),
                    ] {
                        match loader.load::<DataFile<HkxFile>, _>(&p).await {
                            Ok(file) => {
                                behavior_files.push(p);
                                builder
                                    .register_file(&path, (**file).clone())
                                    .map_err(LoadBehaviorError::Build)?;

                                file_loaded = true;
                                break;
                            }
                            Err(err) => {
                                errors.push((p, err));
                            }
                        }
                    }

                    if !file_loaded {
                        return Err(LoadBehaviorError::Load(errors));
                    }
                }
                Err(Hint::NeedSwapGenerator) => {
                    let file = match loader
                        .load::<DataFile<HkxFile>, _>(&descriptor.subgraph_path)
                        .await
                    {
                        Ok(f) => f,
                        Err(err) => {
                            return Err(LoadBehaviorError::Load(vec![(
                                descriptor.subgraph_path,
                                err,
                            )]));
                        }
                    };

                    builder
                        .register_swap_generator((**file).clone())
                        .map_err(LoadBehaviorError::Build)?;
                }
            }
        };

        let clips = Self::load_clips(loader, pool, &descriptor, &graph).await?;

        behavior_files.push(descriptor.behavior_path);
        behavior_files.push(descriptor.subgraph_path);
        for file in &mut behavior_files {
            *file = file
                .strip_prefix(DataPath::from_str("meshes"))
                .unwrap_or(file)
                .to_owned();
        }

        Ok(Self {
            graph,
            behavior_files,
            clips,
        })
    }

    async fn load_clips<T>(
        loader: &AssetLoader,
        pool: &TaskPool,
        descriptor: &ActorBehaviorDescriptor<'_, T>,
        graph: &AnimationGraph,
    ) -> Result<AnimationSet, LoadBehaviorError>
    where
        T: AsRef<DataPath>,
    {
        let animation_files = graph.animation_files();
        let mut tasks = HashMap::with_capacity(animation_files.len());

        for file in animation_files {
            if tasks.contains_key(&file) {
                continue;
            }

            let path = file.to_lowercase().replace("\\", "/").replace("hkt", "hkx");
            let file_stem = path.split("/").last().unwrap_or(&path);
            let search_paths: Vec<_> = descriptor
                .search_paths
                .iter()
                .map(|v| {
                    let mut root = DataPath::from_str("meshes").join(v.as_ref());
                    root.push(&*file_stem);
                    root
                })
                .collect();

            let loader = loader.clone();
            let task = pool.spawn(async move {
                let search_paths: Vec<_> = search_paths.iter().map(|v| v.as_ref()).collect();
                match load_anim(&loader, &search_paths).await {
                    Ok(clip) => Some(clip),
                    Err(()) => {
                        tracing::warn!(
                            "failed to load animation {} in searched paths {:?}",
                            path,
                            search_paths
                        );
                        None
                    }
                }
            });

            tasks.insert(file, task);
        }

        let mut clips = HashMap::new();
        for (file, task) in tasks {
            if let Some(clip) = task.await {
                clips.insert(file, clip);
            }
        }

        Ok(AnimationSet(clips))
    }
}

fn extract_behavior_root_path(file_path: &DataPath) -> &DataPath {
    let mut path = file_path;

    // Remove file part (x.hkx).
    if let Some(p) = path.parent() {
        path = p;
    }

    // Remove behaviors section if present.
    if path.components().last().is_some_and(|v| v == "behaviors") {
        path = path.parent().unwrap();
    }

    path
}

async fn load_anim(
    loader: &AssetLoader,
    search_paths: &[&DataPath],
) -> Result<Handle<FileAnimationClip>, ()> {
    for path in search_paths {
        match loader.load::<FileAnimationClip, _>(*path).await {
            Ok(clip) => return Ok(clip),
            Err(Error::NotFound) => continue,
            Err(err) => {
                tracing::error!("failed to load animation clip: {}", err);
            }
        }
    }

    Err(())
}
