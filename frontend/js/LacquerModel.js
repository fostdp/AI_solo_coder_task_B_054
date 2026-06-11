class LacquerModel {
    constructor(containerId) {
        this.container = document.getElementById(containerId);
        this.scene = null;
        this.camera = null;
        this.renderer = null;
        this.controls = null;
        this.lacquerMesh = null;
        this.strainMesh = null;
        this.sensorMarkers = [];
        this.animationId = null;
        this.mode = 'moisture';
        this.autoRotate = false;
        this.wireframe = false;
        this.strainData = {};
        this.baseVertices = null;
        this.strainBaseVertices = null;
        this._onAnimateCallbacks = [];

        this.init();
    }

    init() {
        const width = this.container.clientWidth;
        const height = this.container.clientHeight;

        this.scene = new THREE.Scene();
        this.scene.background = new THREE.Color(0x0a0e17);
        this.scene.fog = new THREE.Fog(0x0a0e17, 5, 15);

        this.camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 100);
        this.camera.position.set(3, 2, 4);

        this.renderer = new THREE.WebGLRenderer({ antialias: true });
        this.renderer.setSize(width, height);
        this.renderer.setPixelRatio(window.devicePixelRatio);
        this.renderer.shadowMap.enabled = true;
        this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
        this.container.appendChild(this.renderer.domElement);

        this.controls = new THREE.OrbitControls(this.camera, this.renderer.domElement);
        this.controls.enableDamping = true;
        this.controls.dampingFactor = 0.05;
        this.controls.minDistance = 1;
        this.controls.maxDistance = 10;

        this.setupLights();
        this.createLacquerModel();
        this.createStrainMesh();
        this.createSensorMarkers();
        this.createGrid();

        window.addEventListener('resize', () => this.onResize());

        this.animate();
    }

    setupLights() {
        const ambientLight = new THREE.AmbientLight(0x404050, 0.5);
        this.scene.add(ambientLight);

        const mainLight = new THREE.DirectionalLight(0xffffff, 0.8);
        mainLight.position.set(5, 8, 5);
        mainLight.castShadow = true;
        mainLight.shadow.mapSize.width = 2048;
        mainLight.shadow.mapSize.height = 2048;
        this.scene.add(mainLight);

        const fillLight = new THREE.DirectionalLight(0x5dade2, 0.3);
        fillLight.position.set(-3, 2, -3);
        this.scene.add(fillLight);

        const rimLight = new THREE.DirectionalLight(0xaed6f1, 0.2);
        rimLight.position.set(0, 3, -5);
        this.scene.add(rimLight);
    }

    createLacquerModel() {
        const shape = new THREE.Shape();
        
        const points = [
            [0.3, 0],
            [0.6, 0.05],
            [0.8, 0.15],
            [0.9, 0.3],
            [0.85, 0.5],
            [0.7, 0.65],
            [0.5, 0.7],
            [0.3, 0.65],
            [0.15, 0.5],
            [0.1, 0.3],
            [0.15, 0.1],
            [0.3, 0],
        ];

        shape.moveTo(points[0][0], points[0][1]);
        for (let i = 1; i < points.length; i++) {
            shape.lineTo(points[i][0], points[i][1]);
        }

        const extrudeSettings = {
            steps: 32,
            depth: 0.2,
            bevelEnabled: true,
            bevelThickness: 0.02,
            bevelSize: 0.02,
            bevelSegments: 4
        };

        const geometry = new THREE.ExtrudeGeometry(shape, extrudeSettings);
        geometry.center();

        const positions = geometry.attributes.position;
        const colors = new Float32Array(positions.count * 3);

        for (let i = 0; i < positions.count; i++) {
            colors[i * 3] = 0.15;
            colors[i * 3 + 1] = 0.25;
            colors[i * 3 + 2] = 0.35;
        }

        geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));

        const material = new THREE.MeshPhongMaterial({
            vertexColors: true,
            shininess: 30,
            specular: 0x333333,
            transparent: true,
            opacity: 0.9
        });

        this.lacquerMesh = new THREE.Mesh(geometry, material);
        this.lacquerMesh.castShadow = true;
        this.lacquerMesh.receiveShadow = true;
        this.lacquerMesh.rotation.x = -Math.PI / 2;

        this.baseVertices = new Float32Array(positions.array);

        this.scene.add(this.lacquerMesh);
    }

    createStrainMesh() {
        const geometry = new THREE.WireframeGeometry(this.lacquerMesh.geometry);
        this.strainBaseVertices = new Float32Array(geometry.attributes.position.array);

        const material = new THREE.LineBasicMaterial({
            color: 0xe74c3c,
            transparent: true,
            opacity: 0.6
        });

        this.strainMesh = new THREE.LineSegments(geometry, material);
        this.strainMesh.rotation.x = -Math.PI / 2;
        this.strainMesh.visible = false;
        this.strainMesh.scale.set(1.001, 1.001, 1.001);

        this.scene.add(this.strainMesh);
    }

    createSensorMarkers() {
        const moisturePositions = [
            [0.5, 0.3, 0.15],
            [-0.4, 0.25, 0.15],
            [0.3, -0.4, 0.15],
            [-0.3, -0.3, 0.15],
            [0, 0.1, 0.2],
        ];

        const strainPositions = [
            [0.6, 0.1, 0.1],
            [-0.5, 0.15, 0.1],
            [0.4, -0.5, 0.1],
            [-0.4, -0.4, 0.1],
        ];

        moisturePositions.forEach((pos, i) => {
            const marker = this.createSensorMarker('moisture', i);
            marker.position.set(pos[0], pos[1], pos[2]);
            marker.userData = { type: 'moisture', index: i };
            this.sensorMarkers.push(marker);
            this.scene.add(marker);
        });

        strainPositions.forEach((pos, i) => {
            const marker = this.createSensorMarker('strain', i);
            marker.position.set(pos[0], pos[1], pos[2]);
            marker.userData = { type: 'strain', index: i };
            this.sensorMarkers.push(marker);
            this.scene.add(marker);
        });
    }

    createSensorMarker(type, index) {
        const group = new THREE.Group();

        const color = type === 'moisture' ? 0x5dade2 : 0xe74c3c;
        
        const geometry = new THREE.SphereGeometry(0.04, 16, 16);
        const material = new THREE.MeshBasicMaterial({ color });
        const sphere = new THREE.Mesh(geometry, material);
        group.add(sphere);

        const glowGeometry = new THREE.SphereGeometry(0.06, 16, 16);
        const glowMaterial = new THREE.MeshBasicMaterial({
            color,
            transparent: true,
            opacity: 0.3
        });
        const glow = new THREE.Mesh(glowGeometry, glowMaterial);
        group.add(glow);

        group.userData = { type, index, baseScale: 1, pulsePhase: Math.random() * Math.PI * 2 };

        return group;
    }

    createGrid() {
        const gridHelper = new THREE.GridHelper(6, 12, 0x1a2a3a, 0x0f1923);
        gridHelper.position.y = -0.6;
        this.scene.add(gridHelper);

        const axesHelper = new THREE.AxesHelper(1);
        axesHelper.position.set(-2.5, -0.59, -2.5);
        this.scene.add(axesHelper);
    }

    setMode(mode) {
        this.mode = mode;

        switch (mode) {
            case 'moisture':
                this.lacquerMesh.visible = true;
                this.strainMesh.visible = false;
                this.lacquerMesh.material.opacity = 0.9;
                break;
            case 'strain':
                this.lacquerMesh.visible = true;
                this.lacquerMesh.material.opacity = 0.3;
                this.strainMesh.visible = true;
                break;
            case 'both':
                this.lacquerMesh.visible = true;
                this.lacquerMesh.material.opacity = 0.7;
                this.strainMesh.visible = true;
                break;
        }
    }

    updateStrainData(data) {
        this.strainData = data;
        this.updateStrainDeformation();
        this.updateStrainMarkerColors();
    }

    updateStrainMarkerColors() {
        this.sensorMarkers.forEach(marker => {
            const { type, index } = marker.userData;
            if (type === 'strain') {
                const keys = Object.keys(this.strainData);
                const value = this.strainData[keys[index % keys.length]] || 1;
                const hue = Math.min(1, value / 8);
                const color = new THREE.Color().setHSL(0, 0.9, 0.5 + hue * 0.2);
                marker.children[0].material.color = color;
                marker.children[1].material.color = color;
            }
        });
    }

    updateStrainDeformation() {
        if (!this.lacquerMesh || !this.baseVertices) return;

        const strainValues = Object.values(this.strainData);
        const avgStrain = strainValues.length > 0
            ? strainValues.reduce((a, b) => a + b, 0) / strainValues.length
            : 1;

        const deformationFactor = 1 + avgStrain * 0.005;

        const positions = this.lacquerMesh.geometry.attributes.position;
        const baseCount = this.baseVertices.length / 3;

        for (let i = 0; i < baseCount; i++) {
            const x = this.baseVertices[i * 3];
            const y = this.baseVertices[i * 3 + 1];
            const z = this.baseVertices[i * 3 + 2];

            const distFromCenter = Math.sqrt(x * x + y * y);
            const edgeFactor = Math.min(1, distFromCenter * 2);

            const deformation = 1 + edgeFactor * (deformationFactor - 1);

            positions.setXYZ(i, x * deformation, y * deformation, z * deformation);
        }

        positions.needsUpdate = true;
        this.lacquerMesh.geometry.computeVertexNormals();

        if (this.strainMesh && this.strainBaseVertices && this.strainMesh.geometry.attributes.position) {
            const strainPositions = this.strainMesh.geometry.attributes.position;
            const strainCount = this.strainBaseVertices.length / 3;

            for (let i = 0; i < strainCount; i++) {
                const x = this.strainBaseVertices[i * 3];
                const y = this.strainBaseVertices[i * 3 + 1];
                const z = this.strainBaseVertices[i * 3 + 2];

                const distFromCenter = Math.sqrt(x * x + y * y);
                const edgeFactor = Math.min(1, distFromCenter * 2);
                const deformation = 1 + edgeFactor * (deformationFactor - 1);

                strainPositions.setXYZ(i, x * deformation, y * deformation, z * deformation);
            }
            strainPositions.needsUpdate = true;
        }

        const hue = Math.min(1, avgStrain / 10);
        const color = new THREE.Color().setHSL(0, 0.8, 0.5 - hue * 0.2);
        this.strainMesh.material.color = color;
    }

    toggleWireframe() {
        this.wireframe = !this.wireframe;
        this.lacquerMesh.material.wireframe = this.wireframe;
    }

    toggleAutoRotate() {
        this.autoRotate = !this.autoRotate;
        this.controls.autoRotate = this.autoRotate;
        this.controls.autoRotateSpeed = 1;
    }

    resetView() {
        this.camera.position.set(3, 2, 4);
        this.controls.target.set(0, 0, 0);
        this.controls.update();
    }

    zoomIn() {
        this.controls.dollyIn(1.3);
    }

    zoomOut() {
        this.controls.dollyOut(1.3);
    }

    onAnimate(callback) {
        this._onAnimateCallbacks.push(callback);
    }

    animate() {
        this.animationId = requestAnimationFrame(() => this.animate());

        const time = Date.now() * 0.001;

        this.sensorMarkers.forEach(marker => {
            const pulse = 1 + Math.sin(time * 2 + marker.userData.pulsePhase) * 0.15;
            marker.children[1].scale.set(pulse, pulse, pulse);
        });

        this._onAnimateCallbacks.forEach(cb => cb(time));

        this.controls.update();
        this.renderer.render(this.scene, this.camera);
    }

    onResize() {
        const width = this.container.clientWidth;
        const height = this.container.clientHeight;

        this.camera.aspect = width / height;
        this.camera.updateProjectionMatrix();

        this.renderer.setSize(width, height);
    }

    dispose() {
        if (this.animationId) {
            cancelAnimationFrame(this.animationId);
        }
        if (this.renderer) {
            this.renderer.dispose();
        }
    }
}

if (typeof window !== 'undefined') {
    window.LacquerModel = LacquerModel;
}
if (typeof module !== 'undefined' && module.exports) {
    module.exports = LacquerModel;
}
