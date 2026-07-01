"use strict";

class HandleTable {
	constructor() {
		this.handles = {
			0: null,
		};
		this.nextHandle = 1;
	}
	add(obj) {
		const handle = this.nextHandle++;
		this.handles[handle] = obj;
		return handle;
	}
	get(handle) {
		if (!this.has(handle)) {
			throw new Error(`Resource with handle ${handle} not found.`);
		}
		return this.handles[handle];
	}
	remove(handle) {
		let value = this.get(handle);
		delete this.handles[handle];
		return value;
	}
	has(handle) {
		return this.handles.hasOwnProperty(handle);
	}
}

export function createWasmAPI(canvas, options) {
	let memory = null;
	let handles = new HandleTable();
	let decoder = new TextDecoder();
	let encoder = new TextEncoder();
	let gl = canvas.getContext("webgl2", options);
	// gl.drawingBufferColorSpace = "srgb";
	// gl.unpackColorSpace = "srgb";

	function getString(ptr, len) {
		if (ptr === 0) {
			return null;
		}
		return decoder.decode(new Uint8Array(memory.buffer, ptr, len));
	}

	function getTypedView(type, ptr, len) {
		switch (type) {
			case gl.UNSIGNED_BYTE:
				return new Uint8Array(memory.buffer, ptr, len);
			case gl.UNSIGNED_SHORT:
				return new Uint16Array(memory.buffer, ptr, len / 2);
			case gl.UNSIGNED_INT:
			case gl.UNSIGNED_INT_24_8:
				return new Uint32Array(memory.buffer, ptr, len / 4);
			case gl.FLOAT:
				return new Float32Array(memory.buffer, ptr, len / 4);
			case gl.HALF_FLOAT:
				return new Uint16Array(memory.buffer, ptr, len / 2);
			default:
				return new Uint8Array(memory.buffer, ptr, len);
		}
	}

	return {
		bindInstance(instance) {
			memory = instance.exports.memory;
		},
		decodeUtf8(ptr, len) {
			return getString(ptr, len);
		},
		consoleLog(ptr, len) {
			console.log(getString(ptr, len));
		},
		now() { return performance.now(); },
		enable(cap) { gl.enable(cap); },
		disable(cap) { gl.disable(cap); },
		scissor(x, y, width, height) { gl.scissor(x, y, width, height); },
		blendFunc(sfactor, dfactor) { gl.blendFunc(sfactor, dfactor); },
		blendEquation(mode) { gl.blendEquation(mode); },
		depthFunc(func) { gl.depthFunc(func); },
		cullFace(mode) { gl.cullFace(mode); },
		clearColor(r, g, b, a) { gl.clearColor(r, g, b, a); },
		clearDepth(depth) { gl.clearDepth(depth); },
		clearStencil(s) { gl.clearStencil(s); },
		clear(mask) { gl.clear(mask); },
		colorMask(r, g, b, a) { gl.colorMask(r, g, b, a); },
		depthMask(flag) { gl.depthMask(flag); },
		stencilMask(mask) { gl.stencilMask(mask); },
		viewport(x, y, width, height) { gl.viewport(x, y, width, height); },
		createBuffer() { return handles.add(gl.createBuffer()); },
		bindBuffer(target, buffer) { gl.bindBuffer(target, handles.get(buffer)); },
		bufferData(target, size, data_ptr, usage) {
			if (data_ptr === 0) {
				gl.bufferData(target, size, usage);
			}
			else if (memory) {
				const data = new Uint8Array(memory.buffer, data_ptr, size);
				gl.bufferData(target, data, usage);
			}
		},
		bufferSubData(target, offset, size, data_ptr) {
			if (!memory) return;
			const data = new Uint8Array(memory.buffer, data_ptr, size);
			gl.bufferSubData(target, offset, data);
		},
		deleteBuffer(buffer) {
			gl.deleteBuffer(handles.get(buffer));
			handles.remove(buffer);
		},
		enableVertexAttribArray(index) { gl.enableVertexAttribArray(index); },
		disableVertexAttribArray(index) { gl.disableVertexAttribArray(index); },
		vertexAttribPointer(index, size, type, normalized, stride, offset) {
			gl.vertexAttribPointer(index, size, type, normalized, stride, offset);
		},
		vertexAttribIPointer(index, size, type, stride, offset) {
			gl.vertexAttribIPointer(index, size, type, stride, offset);
		},
		vertexAttribDivisor(index, divisor) { gl.vertexAttribDivisor(index, divisor); },
		createProgram() { return handles.add(gl.createProgram()); },
		deleteProgram(program) { let value = handles.remove(program); gl.deleteProgram(value); },
		createShader(type) { return handles.add(gl.createShader(type)); },
		deleteShader(shader) { let value = handles.remove(shader); gl.deleteShader(value); },
		shaderSource(shader, source_ptr, source_len) {
			if (!memory) return;
			const source = decoder.decode(new Uint8Array(memory.buffer, source_ptr, source_len));
			gl.shaderSource(handles.get(shader), source);
		},
		compileShader(shader) { gl.compileShader(handles.get(shader)); },
		getShaderParameter(shader, pname) { return gl.getShaderParameter(handles.get(shader), pname); },
		getShaderInfoLog(shader) {
			const log = gl.getShaderInfoLog(handles.get(shader));
			console.error(`Shader Info Log: ${log}`);
		},
		attachShader(program, shader) { gl.attachShader(handles.get(program), handles.get(shader)); },
		linkProgram(program) { gl.linkProgram(handles.get(program)); },
		useProgram(program) { gl.useProgram(handles.get(program)); },
		getProgramParameter(program, pname) { return gl.getProgramParameter(handles.get(program), pname); },
		getProgramInfoLog(program) {
			const log = gl.getProgramInfoLog(handles.get(program));
			console.error(`Program Info Log: ${log}`);
		},
		getActiveUniform(program, index, bufSize, length_ptr, size_ptr, type_ptr, name_ptr) {
			const info = gl.getActiveUniform(handles.get(program), index);
			if (!info) {
				throw new Error(`No active uniform at index ${index}`);
			}

			const name = info.name;
			const nameBytes = encoder.encode(name);
			if (nameBytes.length >= bufSize) {
				throw new Error(`Uniform name buffer too small: ${bufSize} bytes needed, ${nameBytes.length} bytes provided.`);
			}
			if (!memory) return;
			new Uint8Array(memory.buffer, name_ptr, nameBytes.length).set(nameBytes);
			if (length_ptr) {
				new Uint32Array(memory.buffer, length_ptr, 1)[0] = nameBytes.length;
			}
			if (size_ptr) {
				new Uint32Array(memory.buffer, size_ptr, 1)[0] = info.size;
			}
			if (type_ptr) {
				new Uint32Array(memory.buffer, type_ptr, 1)[0] = info.type;
			}
		},
		getUniformLocation(program, name_ptr, name_len) {
			if (!memory) return 0;
			const name = decoder.decode(new Uint8Array(memory.buffer, name_ptr, name_len));
			const location = gl.getUniformLocation(handles.get(program), name);
			if (!location) {
				throw new Error(`Uniform location for '${name}' not found in program.`);
			}
			return handles.add(location);
		},
		getActiveAttrib(program, index, bufSize, length_ptr, size_ptr, type_ptr, name_ptr) {
			const info = gl.getActiveAttrib(handles.get(program), index);
			if (!info) {
				throw new Error(`No active uniform at index ${index}`);
			}

			const name = info.name;
			const nameBytes = encoder.encode(name);
			if (nameBytes.length >= bufSize) {
				throw new Error(`Attrib name buffer too small: ${bufSize} bytes needed, ${nameBytes.length} bytes provided.`);
			}
			if (!memory) return;
			new Uint8Array(memory.buffer, name_ptr, nameBytes.length).set(nameBytes);
			if (length_ptr) {
				new Uint32Array(memory.buffer, length_ptr, 1)[0] = nameBytes.length;
			}
			if (size_ptr) {
				new Uint32Array(memory.buffer, size_ptr, 1)[0] = info.size;
			}
			if (type_ptr) {
				new Uint32Array(memory.buffer, type_ptr, 1)[0] = info.type;
			}
		},
		getAttribLocation(program, name_ptr, name_len) {
			if (!memory) return -1;
			const name = decoder.decode(new Uint8Array(memory.buffer, name_ptr, name_len));
			return gl.getAttribLocation(handles.get(program), name);
		},
		uniform1fv(location, count, value) {
			if (!memory) return;
			gl.uniform1fv(handles.get(location), new Float32Array(memory.buffer, value, count * 1));
		},
		uniform2fv(location, count, value) {
			if (!memory) return;
			gl.uniform2fv(handles.get(location), new Float32Array(memory.buffer, value, count * 2));
		},
		uniform3fv(location, count, value) {
			if (!memory) return;
			gl.uniform3fv(handles.get(location), new Float32Array(memory.buffer, value, count * 3));
		},
		uniform4fv(location, count, value) {
			if (!memory) return;
			gl.uniform4fv(handles.get(location), new Float32Array(memory.buffer, value, count * 4));
		},
		uniform1iv(location, count, value) {
			if (!memory) return;
			gl.uniform1iv(handles.get(location), new Int32Array(memory.buffer, value, count * 1));
		},
		uniform2iv(location, count, value) {
			if (!memory) return;
			gl.uniform2iv(handles.get(location), new Int32Array(memory.buffer, value, count * 2));
		},
		uniform3iv(location, count, value) {
			if (!memory) return;
			gl.uniform3iv(handles.get(location), new Int32Array(memory.buffer, value, count * 3));
		},
		uniform4iv(location, count, value) {
			if (!memory) return;
			gl.uniform4iv(handles.get(location), new Int32Array(memory.buffer, value, count * 4));
		},
		uniformMatrix2fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix2fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 4));
		},
		uniformMatrix2x3fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix2x3fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 6));
		},
		uniformMatrix2x4fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix2x4fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 8));
		},
		uniformMatrix3x2fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix3x2fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 6));
		},
		uniformMatrix3fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix3fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 9));
		},
		uniformMatrix3x4fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix3x4fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 12));
		},
		uniformMatrix4x2fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix4x2fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 8));
		},
		uniformMatrix4x3fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix4x3fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 12));
		},
		uniformMatrix4fv(location, count, transpose, value) {
			if (!memory) return;
			gl.uniformMatrix4fv(handles.get(location), transpose, new Float32Array(memory.buffer, value, count * 16));
		},
		createTexture() { return handles.add(gl.createTexture()); },
		deleteTexture(texture) {
			let value = handles.remove(texture);
			gl.deleteTexture(value);
		},
		activeTexture(texture) { gl.activeTexture(texture); },
		bindTexture(target, texture) { gl.bindTexture(target, handles.get(texture)); },
		generateMipmap(target) { gl.generateMipmap(target); },
		pixelStorei(pname, param) { gl.pixelStorei(pname, param); },
		texParameteri(target, pname, param) { gl.texParameteri(target, pname, param); },
		texStorage2D(target, levels, internalformat, width, height) { gl.texStorage2D(target, levels, internalformat, width, height); },
		texImage2D(target, level, internalformat, width, height, border, format, type, pixels_ptr, pixels_len) {
			if (!memory) return;
			const pixels = getTypedView(type, pixels_ptr, pixels_len);
			gl.texImage2D(target, level, internalformat, width, height, border, format, type, pixels);
		},
		texSubImage2D(target, level, xoffset, yoffset, width, height, format, type, pixels_ptr, pixels_len) {
			if (!memory) return;
			const pixels = getTypedView(type, pixels_ptr, pixels_len);
			gl.texSubImage2D(target, level, xoffset, yoffset, width, height, format, type, pixels);
		},
		createFramebuffer() { return handles.add(gl.createFramebuffer()); },
		deleteFramebuffer(framebuffer) {
			const value = handles.remove(framebuffer);
			gl.deleteFramebuffer(value);
		},
		bindFramebuffer(target, framebuffer) { gl.bindFramebuffer(target, handles.get(framebuffer)); },
		framebufferTexture2D(target, attachment, textarget, texture, level) {
			gl.framebufferTexture2D(target, attachment, textarget, handles.get(texture), level);
		},
		drawBuffers(n, bufs_ptr) {
			if (!memory) return;
			const bufs = new Uint32Array(memory.buffer, bufs_ptr, n);
			gl.drawBuffers(bufs);
		},
		readBuffer(src) { gl.readBuffer(src); },
		checkFramebufferStatus(target) { return gl.checkFramebufferStatus(target); },
		readPixels(x, y, width, height, format, type, pixels_ptr, pixels_len) {
			if (!memory) return;
			const pixels = getTypedView(type, pixels_ptr, pixels_len);
			gl.readPixels(x, y, width, height, format, type, pixels);
		},
		drawArrays(mode, first, count) { gl.drawArrays(mode, first, count); },
		drawElements(mode, count, type, offset) { gl.drawElements(mode, count, type, offset); },
		drawArraysInstanced(mode, first, count, instancecount) { gl.drawArraysInstanced(mode, first, count, instancecount); },
		drawElementsInstanced(mode, count, type, offset, instancecount) { gl.drawElementsInstanced(mode, count, type, offset, instancecount); },
	}
}
