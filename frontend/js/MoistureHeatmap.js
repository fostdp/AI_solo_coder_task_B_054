class MoistureHeatmap {
    constructor(lacquerModel, options = {}) {
        this.model = lacquerModel;
        this.moistureData = {};
        this.minMoisture = options.minMoisture || 10;
        this.maxMoisture = options.maxMoisture || 90;
    }

    moistureToColor(moisture) {
        const clampedMoisture = Math.max(this.minMoisture, Math.min(this.maxMoisture, moisture));
        const t = (clampedMoisture - this.minMoisture) / (this.maxMoisture - this.minMoisture);

        const r = Math.floor(26 + (212 - 26) * (1 - t)) / 255;
        const g = Math.floor(58 + (230 - 58) * (1 - t * 0.5)) / 255;
        const b = Math.floor(82 + (241 - 82) * (1 - t * 0.3)) / 255;

        return { r, g, b };
    }

    updateData(data) {
        this.moistureData = data;
        this.updateColors();
        this.updateMarkerColors();
    }

    updateColors() {
        if (!this.model.lacquerMesh) return;

        const colors = this.model.lacquerMesh.geometry.attributes.color;
        const positions = this.model.lacquerMesh.geometry.attributes.position;

        const moistureValues = Object.values(this.moistureData);
        const avgMoisture = moistureValues.length > 0 
            ? moistureValues.reduce((a, b) => a + b, 0) / moistureValues.length 
            : 50;

        for (let i = 0; i < positions.count; i++) {
            const x = positions.getX(i);
            const y = positions.getY(i);
            const z = positions.getZ(i);

            const distFromCenter = Math.sqrt(x * x + y * y);
            const surfaceFactor = Math.min(1, distFromCenter * 1.5);
            
            const variation = (Math.sin(x * 5 + z * 3) * 0.3 + 1) * 0.5;
            const localMoisture = avgMoisture + (variation - 0.5) * 20;

            const { r, g, b } = this.moistureToColor(localMoisture);

            colors.setXYZ(i, r, g, b);
        }

        colors.needsUpdate = true;
    }

    updateMarkerColors() {
        if (!this.model.sensorMarkers) return;

        this.model.sensorMarkers.forEach(marker => {
            const { type, index } = marker.userData;
            
            if (type === 'moisture') {
                const keys = Object.keys(this.moistureData);
                const value = this.moistureData[keys[index % keys.length]] || 50;
                const { r, g, b } = this.moistureToColor(value);
                marker.children[0].material.color.setRGB(r, g, b);
                marker.children[1].material.color.setRGB(r, g, b);
            }
        });
    }

    setRange(min, max) {
        this.minMoisture = min;
        this.maxMoisture = max;
    }
}

if (typeof window !== 'undefined') {
    window.MoistureHeatmap = MoistureHeatmap;
}
if (typeof module !== 'undefined' && module.exports) {
    module.exports = MoistureHeatmap;
}
