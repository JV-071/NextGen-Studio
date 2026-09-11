#include "canvas.h"
#include <QPainter>
#include <QMouseEvent>
#include <QWheelEvent>
#include <QRegularExpression>
#include <algorithm>
namespace studio {
Canvas::Canvas(QWidget* parent):QWidget(parent) {setMinimumSize(400,320);setMouseTracking(true);setFocusPolicy(Qt::StrongFocus);}
void Canvas::setDocument(const Document* doc){doc_=doc;layout();update();}
void Canvas::select(int n){selection_=n;update();}
void Canvas::setZoom(double zoom){zoom_=std::clamp(zoom,0.25,2.5);update();}
QPointF Canvas::origin() const{return {32,60};}
void Canvas::layout(){
    boxes_.clear();if(!doc_)return;
    const auto& ns=doc_->nodes();boxes_.resize(ns.size());
    for(int i=0;i<ns.size();++i){
        if(i>3000)break;
        const auto& n=ns[i];if(!n.widget)continue;
        auto number=[&](const QString& key,int fallback){bool ok;int v=doc_->value(i,key).toInt(&ok);return ok?std::clamp(v,-8192,8192):fallback;};
        int w=number("width",n.parent<0?620:140),h=number("height",n.parent<0?400:36);
        const auto size=doc_->value(i,"size").split(QRegularExpression("\\s+"),Qt::SkipEmptyParts);
        if(size.size()==2){bool a,b;int sw=size[0].toInt(&a),sh=size[1].toInt(&b);if(a&&b){w=std::clamp(sw,1,8192);h=std::clamp(sh,1,8192);}}
        QRectF parent=n.parent>=0?boxes_.value(n.parent):QRectF(0,0,900,600);
        int x=number("margin-left",n.parent<0?0:16),y=number("margin-top",n.parent<0?0:42+(i%6)*42);
        if(doc_->value(i,"anchors.right")=="parent.right")x=int(parent.width())-w-number("margin-right",0);
        if(doc_->value(i,"anchors.bottom")=="parent.bottom")y=int(parent.height())-h-number("margin-bottom",0);
        if(doc_->value(i,"anchors.fill")=="parent"){x=0;y=0;w=int(parent.width());h=int(parent.height());}
        if(doc_->value(i,"anchors.centerIn")=="parent"){x=int(parent.width()-w)/2;y=int(parent.height()-h)/2;}
        boxes_[i]=QRectF(parent.topLeft()+QPointF(x,y),QSizeF(std::max(1,w),std::max(1,h)));
    }
}
void Canvas::paintEvent(QPaintEvent*){
    QPainter p(this);p.fillRect(rect(),QColor("#101820"));
    p.setPen(QColor("#25323f"));for(int x=0;x<width();x+=24)for(int y=40;y<height();y+=24)p.drawPoint(x,y);
    p.setPen(QColor("#95a8ba"));p.drawText(QRect(20,12,width()-40,32),Qt::AlignLeft|Qt::AlignVCenter,
        "ESQUEMA DE LAYOUT  ·  Estilos, scripts e imagens: use a prévia nativa");
    p.translate(origin());p.scale(zoom_,zoom_);
    if(!doc_)return;
    for(int i=0;i<boxes_.size()&&i<3000;++i){
        auto box=boxes_[i];if(box.isEmpty())continue;
        const auto& n=doc_->nodes()[i];
        QColor fill(doc_->value(i,"background-color"));if(!fill.isValid())fill=QColor(n.parent<0?"#202d39":"#2a3c4b");
        const auto isLabel=n.name.contains("Label");p.setBrush(isLabel?QBrush(Qt::NoBrush):QBrush(fill));
        p.setPen(QColor("#405363"));if(!isLabel)p.drawRoundedRect(box,3,3);
        p.setPen(QColor("#dce6ee"));
        auto label=doc_->value(i,"text",doc_->value(i,"id",n.name));
        p.drawText(box.adjusted(8,4,-8,-4),n.parent<0?Qt::AlignTop|Qt::AlignLeft:Qt::AlignCenter,label);
        if(i==selection_){p.setBrush(Qt::NoBrush);p.setPen(QPen(QColor("#30c9bf"),1.5/zoom_));p.drawRect(box);
            p.setBrush(QColor("#30c9bf"));p.drawRect(QRectF(box.bottomRight()-QPointF(4,4),QSizeF(8,8)));}
    }
}
void Canvas::mousePressEvent(QMouseEvent* e){
    if(e->button()!=Qt::LeftButton||!doc_)return;
    const auto pos=(e->position()-origin())/zoom_;
    for(int i=std::min(int(boxes_.size()),3000)-1;i>=0;--i)if(boxes_[i].contains(pos)){
        selection_=i;emit selected(i);dragging_=i;dragStart_=pos;dragRect_=boxes_[i];
        resizing_=QRectF(dragRect_.bottomRight()-QPointF(9,9),QSizeF(18,18)).contains(pos);
        // Anchored and layout-managed geometry must not be silently converted to absolute coordinates.
        if(!resizing_){const auto& n=doc_->nodes()[i];for(const auto& pr:n.properties)if(pr.key.startsWith("anchors."))dragging_=-1;
            if(n.parent>=0) for(const auto& pr:doc_->nodes()[n.parent].properties)if(pr.key=="layout")dragging_=-1;
            if(n.parent<0)dragging_=-1;}
        update();return;
    }
}
void Canvas::mouseMoveEvent(QMouseEvent* e){
    if(dragging_<0)return;auto delta=(e->position()-origin())/zoom_-dragStart_;
    auto r=dragRect_;if(resizing_)r.setSize(QSizeF(std::max(8.,r.width()+delta.x()),std::max(8.,r.height()+delta.y())));else r.translate(delta);
    boxes_[dragging_]=r;update();
}
void Canvas::mouseReleaseEvent(QMouseEvent*){
    if(dragging_<0||!doc_)return;const int n=dragging_;dragging_=-1;
    const auto r=boxes_[n];if(r==dragRect_)return;
    const int parent=doc_->nodes()[n].parent;auto pos=r.topLeft()-(parent>=0?boxes_[parent].topLeft():QPointF());
    emit geometryEdited(n,qRound(pos.x()),qRound(pos.y()),qRound(r.width()),qRound(r.height()),resizing_);
}
void Canvas::wheelEvent(QWheelEvent* e){if(e->modifiers().testFlag(Qt::ControlModifier)){setZoom(zoom_*(e->angleDelta().y()>0?1.1:0.9));e->accept();}else QWidget::wheelEvent(e);}
}
