# 3D renderer written in rust with openGL 4.6. 
## Description
With this project i covered almost all LearnOpenGL lessons. 
It is unfinished because the code got messy and i got bored of it. 
## Features
- Multiple light sources
- SSAO (based on scene depth + scene normals)
- Shadow mapping
- Post processing (vignette, chromatic abberation)
## Rendering pipeline
1) Directional light shadow caster pass
2) Depth + normal prepass
3) SSAO compute shader pass
4) SSAO blur compute shader pass
5) Forward lighting pass
6) Post processing pass
## Example screenshot with gizmos turned on
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/de9c077b-95e1-441e-8cb7-18be0a51ebc5" />
## Example screenshot with gizmos turned off
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/23eac39a-f225-47cf-8f7f-b0236a5c6001" />
